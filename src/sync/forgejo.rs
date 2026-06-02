//! Forgejo Sync Adapter
//!
//! Implements synchronization with Forgejo/Gitea instances.

use crate::issue::{Effort, Issue, Status};
use crate::storage::config::SyncConfig;
use crate::sync::{keyring, AuthMode, SyncProvider};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

pub struct ForgejoProvider {
    config: SyncConfig,
    client: Client,
    auth_mode: AuthMode,
}

impl ForgejoProvider {
    pub fn new(config: SyncConfig) -> Self {
        Self::with_auth_mode(config, AuthMode::Interactive)
    }

    pub fn with_auth_mode(config: SyncConfig, auth_mode: AuthMode) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            config,
            client,
            auth_mode,
        }
    }

    fn get_token(&self) -> Result<String> {
        match keyring::get_token(&self.config.url, &self.config.owner) {
            Ok(token) => Ok(token),
            Err(_) if self.auth_mode.allows_prompt() => self.login_interactive(),
            Err(err) => Err(crate::sync::auth_required_error(
                &self.config.provider,
                &self.config.url,
                err,
            )),
        }
    }

    fn login_interactive(&self) -> Result<String> {
        println!("🔒 Authentication required for {}", self.config.url);
        let token = keyring::prompt_for_token(&self.config.url)?;
        keyring::set_token(&self.config.url, &self.config.owner, &token)?;
        Ok(token)
    }

    // API Models

    fn base_url(&self) -> String {
        format!(
            "{}/api/v1/repos/{}/{}",
            self.config.url, self.config.owner, self.config.repo
        )
    }
}

// Forgejo API Models
#[derive(Debug, Serialize, Deserialize)]
struct ForgejoIssue {
    number: i64,
    title: String,
    body: Option<String>,
    state: String, // "open" or "closed"
    labels: Vec<ForgejoLabel>,
    assignee: Option<ForgejoUser>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ForgejoLabel {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ForgejoUser {
    username: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct CreateIssuePayload {
    title: String,
    body: String,
    labels: Option<Vec<i64>>, // Label IDs needed? Or can we create by name? Forgejo usually needs IDs.
                              // For MVP, we might skip labels or try to use names if API supports it (sometimes it does via specialized endpoints or just ignores unknown)
                              // Actually Forgejo/Gitea API for creating issue takes `labels` as []int64.
                              // Simplifying: We will put tags in the body for now or ignore them to start simple.
}

impl SyncProvider for ForgejoProvider {
    fn login(&self) -> Result<()> {
        let _ = self.get_token()?;
        log::info!("✅ Authenticated with Forgejo");
        Ok(())
    }

    fn pull(&self) -> Result<Vec<Issue>> {
        let token = self.get_token()?;
        let url = format!("{}/issues", self.base_url());

        // Fetch open issues
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", token))
            .query(&[("state", "all")])
            .send()
            .context("Failed to fetch issues from Forgejo")?;

        if !response.status().is_success() {
            return Err(anyhow!("API Error: {}", response.status()));
        }

        let api_issues: Vec<ForgejoIssue> = response.json()?;
        let mut issues = Vec::new();

        for api_issue in api_issues {
            let status = if api_issue.state == "closed" {
                Status::Done
            } else {
                Status::Backlog
            };

            let mut remotes = std::collections::HashMap::new();
            remotes.insert(self.config.provider.clone(), api_issue.number.to_string());

            let issue = Issue {
                id: uuid::Uuid::new_v4().to_string(), // New ID for imported issue (dedup logic in main.rs needed)
                title: api_issue.title,
                description: api_issue.body.unwrap_or_default(),
                status,
                effort: Effort::default(),
                tags: api_issue.labels.into_iter().map(|l| l.name).collect(),
                assignee: api_issue.assignee.map(|u| u.username),
                sprint: None,
                due: None,
                started: None,
                completed: None,
                blocked: false,
                created: api_issue.created_at,
                updated: api_issue.updated_at,
                remotes,
                repo: None, // Will be set by multi-repo logic
            };
            issues.push(issue);
        }

        Ok(issues)
    }

    fn push(&self, issues: &mut [Issue]) -> Result<()> {
        let token = self.get_token()?;
        let base_url = format!("{}/issues", self.base_url());

        // log::info!("📤 Synching {} issues with Forgejo...", issues.len());

        for issue in issues {
            let payload = serde_json::json!({
                "title": issue.title,
                "body": issue.description,
                "closed": matches!(issue.status, Status::Done),
                "assignee": issue.assignee.as_deref().unwrap_or(""),
            });

            // Check if issue is already linked
            if let Some(remote_id) = issue.remotes.get(&self.config.provider) {
                // UPDATE existing issue
                let url = format!("{}/{}", base_url, remote_id);
                // log::info!("   Updating #{}: title='{}'", remote_id, issue.title);

                // Forgejo API: PATCH to update
                let response = self
                    .client
                    .patch(&url)
                    .header("Authorization", format!("token {}", token))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .context(format!("Failed to update issue #{}", remote_id))?;

                let status = response.status();
                let resp_body = response.text().unwrap_or_default();
                if !status.is_success() {
                    log::error!("   ⚠️ Update failed ({}): {}", status, resp_body);
                }
            } else {
                // CREATE new issue
                // log::info!("   Creating '{}'...", issue.title); // verbose

                let response = self
                    .client
                    .post(&base_url)
                    .header("Authorization", format!("token {}", token))
                    .json(&payload)
                    .send()
                    .context(format!("Failed to create issue: {}", issue.title))?;

                if response.status().is_success() {
                    let created_issue: ForgejoIssue = response.json()?;
                    // Link local issue to remote
                    issue.remotes.insert(
                        self.config.provider.clone(),
                        created_issue.number.to_string(),
                    );
                    // log::info!("   Linked to #{}", created_issue.number);
                }
            }
        }

        Ok(())
    }

    fn delete_missing(&self, local_issues: &[Issue]) -> Result<usize> {
        let token = self.get_token()?;
        // specific to Forgejo/Gitea: delete is at /repos/{owner}/{repo}/issues/{index}
        // base_url() returns .../api/v1/repos/{owner}/{repo}
        // so we need to construct the url correctly.
        // The pull() used .../issues, so deletion is .../issues/{index}
        let url_base = format!("{}/issues", self.base_url());

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
                .header("Authorization", format!("token {}", token))
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

    // Merge Request operations (Pull Requests in Forgejo/Gitea)
    fn create_mr(&self, mr: &crate::mr::MergeRequest) -> Result<u64> {
        let token = self.get_token()?;
        let url = format!("{}/pulls", self.base_url());

        let payload = serde_json::json!({
            "title": mr.title,
            "head": mr.source_branch,
            "base": mr.target_branch,
            "body": mr.description,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context("Failed to create pull request")?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to create PR: {}", error_text));
        }

        // Try to parse response, with better error message on failure
        let body = response.text().context("Failed to read response body")?;
        let pr: ForgejoPullRequest = serde_json::from_str(&body)
            .context(format!("Failed to parse PR response: {}", body))?;
        Ok(pr.number as u64)
    }

    fn list_mrs(&self) -> Result<Vec<crate::mr::MergeRequest>> {
        let token = self.get_token()?;
        let url = format!("{}/pulls", self.base_url());

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", token))
            .query(&[("state", "open")])
            .send()
            .context("Failed to fetch pull requests")?;

        if !response.status().is_success() {
            return Err(anyhow!("API Error: {}", response.status()));
        }

        // Parse with better error handling
        let body = response.text().context("Failed to read response")?;
        let prs: Vec<ForgejoPullRequest> =
            serde_json::from_str(&body).context(format!("Failed to parse PRs: {}", body))?;
        let mut mrs = Vec::new();

        for pr in prs {
            let state = if pr.merged {
                crate::mr::MRState::Merged
            } else if pr.state == "closed" {
                crate::mr::MRState::Closed
            } else {
                crate::mr::MRState::Open
            };

            let mr = crate::mr::MergeRequest {
                id: uuid::Uuid::new_v4().to_string(),
                remote_id: Some(pr.number as u64),
                source_branch: pr.head.ref_name,
                target_branch: pr.base.ref_name,
                title: pr.title,
                description: pr.body.unwrap_or_default(),
                state,
                author: pr.user.map(|u| u.username),
                assignees: pr
                    .assignees
                    .unwrap_or_default()
                    .into_iter()
                    .map(|a| a.username)
                    .collect(),
                labels: pr
                    .labels
                    .unwrap_or_default()
                    .into_iter()
                    .map(|l| l.name)
                    .collect(),
                linked_issues: Vec::new(),
                web_url: Some(pr.html_url),
                created: pr.created_at,
                updated: pr.updated_at,
                merged_at: pr.merged_at,
                is_draft: pr.draft.unwrap_or(false),
                approvals: 0,
                upvotes: 0,
                downvotes: 0,
                pipeline_status: None,
            };
            mrs.push(mr);
        }

        Ok(mrs)
    }

    fn get_mr(&self, remote_id: u64) -> Result<crate::mr::MergeRequest> {
        let token = self.get_token()?;
        let url = format!("{}/pulls/{}", self.base_url(), remote_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", token))
            .send()
            .context(format!("Failed to fetch PR #{}", remote_id))?;

        if !response.status().is_success() {
            return Err(anyhow!("API Error: {}", response.status()));
        }

        let pr: ForgejoPullRequest = response.json()?;

        let state = if pr.merged {
            crate::mr::MRState::Merged
        } else if pr.state == "closed" {
            crate::mr::MRState::Closed
        } else {
            crate::mr::MRState::Open
        };

        Ok(crate::mr::MergeRequest {
            id: uuid::Uuid::new_v4().to_string(),
            remote_id: Some(pr.number as u64),
            source_branch: pr.head.ref_name,
            target_branch: pr.base.ref_name,
            title: pr.title,
            description: pr.body.unwrap_or_default(),
            state,
            author: pr.user.map(|u| u.username),
            assignees: pr
                .assignees
                .unwrap_or_default()
                .into_iter()
                .map(|a| a.username)
                .collect(),
            labels: pr
                .labels
                .unwrap_or_default()
                .into_iter()
                .map(|l| l.name)
                .collect(),
            linked_issues: Vec::new(),
            web_url: Some(pr.html_url),
            created: pr.created_at,
            updated: pr.updated_at,
            merged_at: pr.merged_at,
            is_draft: pr.draft.unwrap_or(false),
            approvals: 0,
            upvotes: 0,
            downvotes: 0,
            pipeline_status: None,
        })
    }

    fn update_mr(&self, mr: &crate::mr::MergeRequest) -> Result<()> {
        if let Some(remote_id) = mr.remote_id {
            let token = self.get_token()?;
            let url = format!("{}/pulls/{}", self.base_url(), remote_id);

            let payload = serde_json::json!({
                "title": mr.title,
                "body": mr.description,
            });

            let response = self
                .client
                .patch(&url)
                .header("Authorization", format!("token {}", token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .context(format!("Failed to update PR #{}", remote_id))?;

            if !response.status().is_success() {
                let error_text = response.text().unwrap_or_default();
                return Err(anyhow!("Failed to update PR: {}", error_text));
            }

            Ok(())
        } else {
            Err(anyhow!("Cannot update MR without remote_id"))
        }
    }

    fn merge_mr(&self, remote_id: u64) -> Result<()> {
        let token = self.get_token()?;
        let url = format!("{}/pulls/{}/merge", self.base_url(), remote_id);

        let payload = serde_json::json!({
            "Do": "merge",
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context(format!("Failed to merge PR #{}", remote_id))?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to merge PR: {}", error_text));
        }

        Ok(())
    }

    fn close_mr(&self, remote_id: u64) -> Result<()> {
        let token = self.get_token()?;
        let url = format!("{}/pulls/{}", self.base_url(), remote_id);

        let payload = serde_json::json!({
            "state": "closed",
        });

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("token {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context(format!("Failed to close PR #{}", remote_id))?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to close PR: {}", error_text));
        }

        Ok(())
    }

    fn approve_mr(&self, remote_id: u64) -> Result<()> {
        let token = self.get_token()?;
        let url = format!("{}/pulls/{}/reviews", self.base_url(), remote_id);

        let payload = serde_json::json!({
            "event": "APPROVED",
            "body": "Approved via ProGit TUI",
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context(format!("Failed to approve PR #{}", remote_id))?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to approve PR: {}", error_text));
        }

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

        const PROVIDER: &str = "forgejo";

        // Filter comments that still need pushing.
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

        // Build the comment payload, skipping comments whose anchor is no
        // longer valid (option 4a from the sign-off: skip + warn, never abort).
        // Track which slice index each payload entry came from so we can
        // write back the remote ID after the response comes in.
        let mut payload_comments: Vec<serde_json::Value> = Vec::new();
        let mut payload_to_slice: Vec<usize> = Vec::new();

        for &idx in &pending_indices {
            let c = &comments[idx];
            match position::resolve(&repo, &c.file_path, c.line_number, &c.commit_sha) {
                Ok(_pos) => {
                    payload_comments.push(serde_json::json!({
                        "body": c.text,
                        "path": c.file_path,
                        "new_position": c.line_number,
                        "old_position": 0,
                    }));
                    payload_to_slice.push(idx);
                }
                Err(e) => {
                    log::warn!(
                        "skipping review comment {} on {}:{} — {}",
                        c.id,
                        c.file_path,
                        c.line_number,
                        e
                    );
                }
            }
        }

        if payload_comments.is_empty() {
            log::info!("No resolvable review comments to push to Forgejo.");
            return Ok(0);
        }

        let token = self.get_token()?;
        let url = format!("{}/pulls/{}/reviews", self.base_url(), mr_remote_id);

        let body = serde_json::json!({
            "body": review.summary.clone().unwrap_or_default(),
            "commit_id": review.commit_sha,
            "event": "COMMENT",
            "comments": payload_comments,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .context(format!("Failed POST /pulls/{}/reviews", mr_remote_id))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Forgejo refused review (status {}): {}",
                status,
                err_body
            ));
        }

        // The response is a PullReview with `id` but does NOT inline its
        // comments. Follow up with GET /reviews/{id}/comments to retrieve
        // their forge IDs, then match by index (Forgejo preserves order).
        let review_resp: serde_json::Value = response
            .json()
            .context("Failed to decode Forgejo review response")?;
        let review_id = review_resp
            .get("id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow!("Forgejo review response missing `id`"))?;

        let comments_url = format!(
            "{}/pulls/{}/reviews/{}/comments",
            self.base_url(),
            mr_remote_id,
            review_id
        );
        let listed: Vec<serde_json::Value> = self
            .client
            .get(&comments_url)
            .header("Authorization", format!("token {}", token))
            .send()
            .context("Failed GET /reviews/{id}/comments")?
            .json()
            .context("Failed to decode Forgejo review comments list")?;

        if listed.len() != payload_to_slice.len() {
            log::warn!(
                "Forgejo returned {} comments for {} pushed; matching by index anyway",
                listed.len(),
                payload_to_slice.len()
            );
        }

        let mut filled = 0usize;
        for (response_idx, slice_idx) in payload_to_slice.iter().enumerate() {
            let Some(returned) = listed.get(response_idx) else {
                continue;
            };
            if let Some(remote_id) = returned.get("id").and_then(|v| v.as_i64()) {
                comments[*slice_idx]
                    .external_ids
                    .insert(PROVIDER.to_string(), remote_id.to_string());
                filled += 1;
            }
        }

        Ok(filled)
    }
}

// Forgejo Pull Request API Models
#[derive(Debug, Deserialize)]
struct ForgejoPullRequest {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    merged: bool,
    draft: Option<bool>,
    html_url: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    user: Option<ForgejoUser>,
    assignees: Option<Vec<ForgejoUser>>,
    labels: Option<Vec<ForgejoLabel>>,
    head: ForgejoBranch,
    base: ForgejoBranch,
}

#[derive(Debug, Deserialize)]
struct ForgejoBranch {
    #[serde(rename = "ref")]
    ref_name: String,
}
