use super::{keyring, SyncProvider};
use crate::issue::{Effort, Issue, Status};
use crate::storage::config::SyncConfig;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GitLab Provider Implementation
pub struct GitLabProvider {
    config: SyncConfig,
    client: Client,
}

impl GitLabProvider {
    pub fn new(config: SyncConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { config, client }
    }

    fn get_token(&self) -> Result<String> {
        // Try keyring first
        if let Ok(token) = keyring::get_token(&self.config.url, &self.config.owner) {
            return Ok(token);
        }

        // If interactive, prompt
        self.login_interactive()
    }

    fn login_interactive(&self) -> Result<String> {
        println!("🔒 Authentication required for {}", self.config.url);
        let token = keyring::prompt_for_token(&self.config.url)?;
        keyring::set_token(&self.config.url, &self.config.owner, &token)?;
        Ok(token)
    }

    // Convert project path (owner/repo) to URL or ID
    // GitLab uses integer IDs often, but also URL encoding
    fn api_url(&self, path: &str) -> String {
        let base = if self.config.url.ends_with('/') {
            &self.config.url[..self.config.url.len() - 1]
        } else {
            &self.config.url
        };
        // Encode project path: owner/repo -> owner%2Frepo
        let project_path = format!("{}/{}", self.config.owner, self.config.repo);
        let encoded_path = urlencoding::encode(&project_path);

        format!("{}/api/v4/projects/{}/{}", base, encoded_path, path)
    }

    /// Lookup GitLab user ID by username
    fn lookup_user_id(&self, username: &str) -> Result<Option<i64>> {
        let token = self.get_token()?;
        let base = if self.config.url.ends_with('/') {
            &self.config.url[..self.config.url.len() - 1]
        } else {
            &self.config.url
        };

        let url = format!(
            "{}/api/v4/users?username={}",
            base,
            urlencoding::encode(username)
        );

        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to lookup user")?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let users: Vec<GitLabUser> = response.json().unwrap_or_default();
        Ok(users.first().map(|u| u.id))
    }
}

// GitLab API Models
#[derive(Debug, Serialize, Deserialize)]
struct GitLabIssue {
    iid: i64,        // Internal ID (visible to user)
    project_id: i64, // Global Project ID
    title: String,
    description: Option<String>,
    state: String, // "opened", "closed"
    labels: Vec<String>,
    assignee: Option<GitLabUser>,
    assignees: Option<Vec<GitLabUser>>, // GitLab supports multiple
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    web_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitLabUser {
    id: i64,
    name: String,
    username: String,
}

impl SyncProvider for GitLabProvider {
    fn login(&self) -> Result<()> {
        let token = self.get_token()?;

        // Verify token with simple user call
        let url = format!("{}/api/v4/user", self.config.url);
        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to connect to GitLab")?;

        if !response.status().is_success() {
            // If unauthorized, delete token and retry
            if response.status().as_u16() == 401 {
                log::warn!("⚠️  Token invalid or expired.");
                keyring::delete_token(&self.config.url, &self.config.owner)?;
                return self.login(); // Recursive retry with prompt
            }
            return Err(anyhow!(
                "GitLab authentication failed: {}",
                response.status()
            ));
        }

        // log::info!("✅ Authenticated with GitLab");
        Ok(())
    }

    fn pull(&self) -> Result<Vec<Issue>> {
        let token = self.get_token()?;
        let url = self.api_url("issues?state=all&per_page=100"); // Fetch all issues

        // log::info!("📥 Fetching issues from GitLab...");

        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to fetch issues")?;

        let gl_issues: Vec<GitLabIssue> =
            response.json().context("Failed to parse GitLab response")?;

        let mut issues = Vec::new();

        for gl_issue in gl_issues {
            let status = match gl_issue.state.as_str() {
                "closed" => Status::Done,
                _ => Status::Backlog, // Default to backlog for open
            };

            let mut remotes = HashMap::new();
            remotes.insert(self.config.provider.clone(), gl_issue.iid.to_string());

            // Handle assignees (take first one)
            let assignee = gl_issue
                .assignees
                .as_ref()
                .and_then(|a| a.first())
                .map(|u| u.username.clone())
                .or_else(|| gl_issue.assignee.map(|u| u.username));

            let issue = Issue {
                id: uuid::Uuid::new_v4().to_string(),
                title: gl_issue.title,
                description: gl_issue.description.unwrap_or_default(),
                status,
                effort: Effort::default(),
                tags: gl_issue.labels,
                assignee,
                sprint: None,
                due: None,
                started: None,
                completed: None,
                blocked: false,
                created: gl_issue.created_at,
                updated: gl_issue.updated_at,
                remotes,
                repo: None, // Will be set by multi-repo logic
            };
            issues.push(issue);
        }

        Ok(issues)
    }

    fn push(&self, issues: &mut [Issue]) -> Result<()> {
        let token = self.get_token()?;
        let url_base = self.api_url("issues");

        // log::info!("📤 Synching {} issues with GitLab...", issues.len());

        for issue in issues {
            // GitLab expects specific state_event to close/reopen
            let state_event = if matches!(issue.status, Status::Done) {
                "close"
            } else {
                "reopen"
            };

            let mut payload = serde_json::json!({
                "title": issue.title,
                "description": issue.description,
                "labels": issue.tags.join(","),
                "state_event": state_event
            });

            // Add optional fields if present
            if let Some(ref assignee) = issue.assignee {
                // Lookup user ID by username
                if let Ok(Some(user_id)) = self.lookup_user_id(assignee) {
                    payload["assignee_id"] = serde_json::json!(user_id);
                } else {
                    // If lookup fails, try username field (some GitLab versions support it)
                    payload["assignee_ids"] = serde_json::json!([assignee]);
                }
            }
            if let Some(due) = issue.due {
                payload["due_date"] = serde_json::json!(due.format("%Y-%m-%d").to_string());
            }

            if let Some(remote_id) = issue.remotes.get(&self.config.provider) {
                // UPDATE
                let url = format!("{}/{}", url_base, remote_id);
                log::debug!("   Updating #{}: title='{}'", remote_id, issue.title);

                let response = self
                    .client
                    .put(&url)
                    .header("PRIVATE-TOKEN", &token)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .context(format!("Failed to update issue #{}", remote_id))?;

                if !response.status().is_success() {
                    log::error!(
                        "   ⚠️ Update failed: {}",
                        response.text().unwrap_or_default()
                    );
                }
            } else {
                // CREATE
                // log::info!("   Creating '{}'...", issue.title);

                let response = self
                    .client
                    .post(&url_base)
                    .header("PRIVATE-TOKEN", &token)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .context(format!("Failed to create issue: {}", issue.title))?;

                if response.status().is_success() {
                    let created: GitLabIssue = response.json()?;
                    issue
                        .remotes
                        .insert(self.config.provider.clone(), created.iid.to_string());
                } else {
                    log::error!(
                        "   ⚠️ Create failed: {}",
                        response.text().unwrap_or_default()
                    );
                }
            }
        }

        Ok(())
    }

    fn delete_missing(&self, local_issues: &[Issue]) -> Result<usize> {
        let token = self.get_token()?;
        let url_base = self.api_url("issues");

        // Get all remote issue IIDs
        let remote_issues = self.pull()?;
        let remote_ids: std::collections::HashSet<String> = remote_issues
            .iter()
            .filter_map(|i| i.remotes.get(&self.config.provider).cloned())
            .collect();

        // Get local issue IIDs (for this provider)
        let local_ids: std::collections::HashSet<String> = local_issues
            .iter()
            .filter_map(|i| i.remotes.get(&self.config.provider).cloned())
            .collect();

        // IDs to delete = remote - local
        let to_delete: Vec<_> = remote_ids.difference(&local_ids).collect();

        let mut deleted = 0;
        for iid in to_delete {
            let url = format!("{}/{}", url_base, iid);
            // log::info!("   🗑️  Deleting remote issue #{}", iid);

            let response = self
                .client
                .delete(&url)
                .header("PRIVATE-TOKEN", &token)
                .send()
                .context(format!("Failed to delete issue #{}", iid))?;

            if response.status().is_success() {
                deleted += 1;
            } else {
                log::error!(
                    "   ⚠️ Delete failed: {}",
                    response.text().unwrap_or_default()
                );
            }
        }

        Ok(deleted)
    }

    // Merge Request operations
    fn create_mr(&self, mr: &crate::mr::MergeRequest) -> Result<u64> {
        let token = self.get_token()?;
        let url = self.api_url("merge_requests");

        let mut payload = serde_json::json!({
            "source_branch": mr.source_branch,
            "target_branch": mr.target_branch,
            "title": mr.title,
            "description": mr.description,
        });

        // Add optional fields
        if !mr.assignees.is_empty() {
            // GitLab wants assignee IDs, but we'll try usernames first
            // (might need user ID lookup like in push)
            payload["assignee_ids"] = serde_json::json!(mr.assignees);
        }
        if !mr.labels.is_empty() {
            payload["labels"] = serde_json::json!(mr.labels.join(","));
        }
        if mr.is_draft {
            payload["title"] = serde_json::json!(format!("Draft: {}", mr.title));
        }

        log::info!(
            "🔀 Creating MR: {} -> {}",
            mr.source_branch,
            mr.target_branch
        );

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", &token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context("Failed to create merge request")?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("MR creation failed: {}", error_text));
        }

        let created: serde_json::Value = response.json()?;
        let iid = created["iid"]
            .as_u64()
            .ok_or_else(|| anyhow!("No IID in response"))?;

        log::info!("✅ Created MR !{}", iid);
        Ok(iid)
    }

    fn list_mrs(&self) -> Result<Vec<crate::mr::MergeRequest>> {
        let token = self.get_token()?;
        let url = self.api_url("merge_requests?state=opened&per_page=100");

        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to list merge requests")?;

        if !response.status().is_success() {
            return Err(anyhow!("API Error: {}", response.status()));
        }

        let body = response.text().context("Failed to get response body")?;
        let gl_mrs: Vec<serde_json::Value> = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => return Err(anyhow!("Failed to parse MR list: {}\nBody: {}", e, body)),
        };

        let mut mrs = Vec::new();
        for gl_mr in gl_mrs {
            let created_at = gl_mr["created_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let updated_at = gl_mr["updated_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let state_str = gl_mr["state"].as_str().unwrap_or("opened");
            let state = match state_str {
                "opened" | "open" => crate::mr::MRState::Open,
                "merged" => crate::mr::MRState::Merged,
                "closed" => crate::mr::MRState::Closed,
                "locked" => crate::mr::MRState::Closed, // Map locked to closed for now
                _ => crate::mr::MRState::Open,
            };

            let mr = crate::mr::MergeRequest {
                id: uuid::Uuid::new_v4().to_string(),
                remote_id: gl_mr["iid"].as_u64(),
                source_branch: gl_mr["source_branch"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                target_branch: gl_mr["target_branch"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                title: gl_mr["title"].as_str().unwrap_or_default().to_string(),
                description: gl_mr["description"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                state,
                author: gl_mr["author"]["username"].as_str().map(|s| s.to_string()),
                assignees: vec![], // TODO
                labels: vec![],    // TODO
                linked_issues: vec![],
                web_url: gl_mr["web_url"].as_str().map(|s| s.to_string()),
                created: created_at,
                updated: updated_at,
                merged_at: None,
                is_draft: gl_mr["draft"].as_bool().unwrap_or(false)
                    || gl_mr["work_in_progress"].as_bool().unwrap_or(false),
                approvals: gl_mr["upvotes"].as_u64().unwrap_or(0) as u32, // Use upvotes as proxy for now
                upvotes: gl_mr["upvotes"].as_u64().unwrap_or(0) as u32,
                downvotes: gl_mr["downvotes"].as_u64().unwrap_or(0) as u32,
                pipeline_status: None,
            };
            mrs.push(mr);
        }

        Ok(mrs)
    }

    fn get_mr(&self, remote_id: u64) -> Result<crate::mr::MergeRequest> {
        let token = self.get_token()?;
        let url = self.api_url(&format!("merge_requests/{}", remote_id));

        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to get merge request")?;

        let gl_mr: serde_json::Value = response.json()?;

        Ok(crate::mr::MergeRequest {
            id: uuid::Uuid::new_v4().to_string(),
            remote_id: Some(remote_id),
            source_branch: gl_mr["source_branch"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            target_branch: gl_mr["target_branch"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            title: gl_mr["title"].as_str().unwrap_or_default().to_string(),
            description: gl_mr["description"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            state: crate::mr::MRState::Open,
            author: gl_mr["author"]["username"].as_str().map(|s| s.to_string()),
            assignees: vec![],
            labels: vec![],
            linked_issues: vec![],
            web_url: gl_mr["web_url"].as_str().map(|s| s.to_string()),
            created: gl_mr["created_at"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(chrono::Utc::now),
            updated: gl_mr["updated_at"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(chrono::Utc::now),
            merged_at: None,
            is_draft: gl_mr["draft"].as_bool().unwrap_or(false),
            approvals: gl_mr["upvotes"].as_u64().unwrap_or(0) as u32,
            upvotes: gl_mr["upvotes"].as_u64().unwrap_or(0) as u32,
            downvotes: gl_mr["downvotes"].as_u64().unwrap_or(0) as u32,
            pipeline_status: None,
        })
    }

    fn update_mr(&self, mr: &crate::mr::MergeRequest) -> Result<()> {
        let Some(remote_id) = mr.remote_id else {
            return Err(anyhow!("Cannot update MR without remote_id"));
        };

        let token = self.get_token()?;
        let url = self.api_url(&format!("merge_requests/{}", remote_id));

        let payload = serde_json::json!({
            "title": mr.title,
            "description": mr.description,
        });

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context("Failed to update merge request")?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("MR update failed: {}", error_text));
        }

        Ok(())
    }

    fn approve_mr(&self, remote_id: u64) -> Result<()> {
        let token = self.get_token()?;
        let url = self.api_url(&format!("merge_requests/{}/approve", remote_id));

        log::info!("👍 Approving MR !{}", remote_id);

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to approve merge request")?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("MR approval failed: {}", error_text));
        }

        log::info!("✅ Approved MR !{}", remote_id);
        Ok(())
    }

    fn merge_mr(&self, remote_id: u64) -> Result<()> {
        let token = self.get_token()?;
        let url = self.api_url(&format!("merge_requests/{}/merge", remote_id));

        log::info!("🔀 Merging MR !{}", remote_id);

        // GitLab specific parameters for merge
        let payload = serde_json::json!({
            "should_remove_source_branch": true,
            "merge_when_pipeline_succeeds": true,
        });

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context("Failed to merge merge request")?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("MR merge failed: {}", error_text));
        }

        log::info!("✅ Merged MR !{}", remote_id);
        Ok(())
    }

    fn close_mr(&self, remote_id: u64) -> Result<()> {
        let token = self.get_token()?;
        let url = self.api_url(&format!("merge_requests/{}", remote_id));

        log::info!("🚫 Closing MR !{}", remote_id);

        let payload = serde_json::json!({
            "state_event": "close"
        });

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context("Failed to close merge request")?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("MR close failed: {}", error_text));
        }

        log::info!("✅ Closed MR !{}", remote_id);
        Ok(())
    }

    fn push_review_comments(
        &self,
        repo_path: &std::path::Path,
        mr_remote_id: u64,
        review: &crate::review::Review,
        comments: &mut [crate::review::ReviewComment],
    ) -> Result<usize> {
        use crate::review_sync::position;

        const PROVIDER: &str = "gitlab";

        let pending_indices: Vec<usize> = comments
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.external_ids.contains_key(PROVIDER))
            .map(|(i, _)| i)
            .collect();

        if pending_indices.is_empty() {
            return Ok(0);
        }

        let repo = git2::Repository::open(repo_path)
            .context("Failed to open git repository for review-comment push")?;
        let token = self.get_token()?;

        // GitLab needs base/start/head SHAs from the MR's diff_refs.
        // Fetch the MR once and reuse the SHAs across all comment posts.
        let mr_url = self.api_url(&format!("merge_requests/{}", mr_remote_id));
        let mr_resp: serde_json::Value = self
            .client
            .get(&mr_url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context(format!("Failed GET /merge_requests/{}", mr_remote_id))?
            .json()
            .context("Failed to decode GitLab MR response")?;

        let diff_refs = mr_resp
            .get("diff_refs")
            .ok_or_else(|| anyhow!("GitLab MR response missing diff_refs"))?;
        let base_sha = diff_refs
            .get("base_sha")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("diff_refs.base_sha missing"))?
            .to_string();
        let start_sha = diff_refs
            .get("start_sha")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("diff_refs.start_sha missing"))?
            .to_string();
        let head_sha = diff_refs
            .get("head_sha")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("diff_refs.head_sha missing"))?
            .to_string();

        let discussions_url = self.api_url(&format!(
            "merge_requests/{}/discussions",
            mr_remote_id
        ));

        let mut filled = 0usize;
        for &idx in &pending_indices {
            let c = &comments[idx];
            // Anchor verification — the user's commit_sha may be older
            // than diff_refs.head_sha, but we still want to verify the
            // line existed at the user's anchor. Skip + warn on failure.
            if let Err(e) = position::resolve(&repo, &c.file_path, c.line_number, &c.commit_sha)
            {
                log::warn!(
                    "skipping review comment {} on {}:{} — {}",
                    c.id,
                    c.file_path,
                    c.line_number,
                    e
                );
                continue;
            }

            let body = serde_json::json!({
                "body": c.text,
                "position": {
                    "position_type": "text",
                    "base_sha": base_sha,
                    "start_sha": start_sha,
                    "head_sha": head_sha,
                    "new_path": c.file_path,
                    "new_line": c.line_number,
                },
            });

            let resp = self
                .client
                .post(&discussions_url)
                .header("PRIVATE-TOKEN", &token)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .context(format!(
                    "Failed POST /merge_requests/{}/discussions",
                    mr_remote_id
                ))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let err_body = resp.text().unwrap_or_default();
                log::warn!(
                    "GitLab refused comment {} (status {}): {}",
                    c.id,
                    status,
                    err_body
                );
                continue;
            }

            let json: serde_json::Value = resp
                .json()
                .context("Failed to decode GitLab discussion response")?;

            // The comment ID we want is notes[0].id — the actual note,
            // not the parent discussion. Fall back to the discussion ID
            // if notes is empty (shouldn't happen for line comments).
            let note_id = json
                .get("notes")
                .and_then(|n| n.as_array())
                .and_then(|arr| arr.first())
                .and_then(|n| n.get("id"))
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
                .or_else(|| {
                    json.get("id")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                });

            if let Some(rid) = note_id {
                comments[idx]
                    .external_ids
                    .insert(PROVIDER.to_string(), rid);
                filled += 1;
            } else {
                log::warn!(
                    "GitLab discussion response for comment {} had no usable id",
                    c.id
                );
            }
        }

        // `review` is unused for GitLab — there's no parallel concept of a
        // top-level review session; each line comment is its own discussion.
        // Tell the compiler we know.
        let _ = review;

        Ok(filled)
    }
}
