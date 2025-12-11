use super::{SyncProvider, keyring};
use crate::issue::{Issue, Status, Effort};
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
             
        Self {
            config,
            client,
        }
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
            &self.config.url[..self.config.url.len()-1]
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
            &self.config.url[..self.config.url.len()-1]
        } else {
            &self.config.url
        };
        
        let url = format!("{}/api/v4/users?username={}", base, urlencoding::encode(username));
        
        let response = self.client.get(&url)
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
    iid: i64,          // Internal ID (visible to user)
    project_id: i64,   // Global Project ID
    title: String,
    description: Option<String>,
    state: String,     // "opened", "closed"
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
        let response = self.client.get(&url)
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
            return Err(anyhow!("GitLab authentication failed: {}", response.status()));
        }
        
        // log::info!("✅ Authenticated with GitLab");
        Ok(())
    }

    fn pull(&self) -> Result<Vec<Issue>> {
        let token = self.get_token()?;
        let url = self.api_url("issues?state=all&per_page=100"); // Fetch all issues
        
        // log::info!("📥 Fetching issues from GitLab...");
        
        let response = self.client.get(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to fetch issues")?;
            
        let gl_issues: Vec<GitLabIssue> = response.json()
            .context("Failed to parse GitLab response")?;
            
        let mut issues = Vec::new();
        
        for gl_issue in gl_issues {
            let status = match gl_issue.state.as_str() {
                "closed" => Status::Done,
                _ => Status::Backlog, // Default to backlog for open
            };
            
            let mut remotes = HashMap::new();
            remotes.insert(self.config.provider.clone(), gl_issue.iid.to_string());
            
            // Handle assignees (take first one)
            let assignee = gl_issue.assignees.as_ref()
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
                repo: None,  // Will be set by multi-repo logic
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
                
                let response = self.client.put(&url)
                    .header("PRIVATE-TOKEN", &token)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .context(format!("Failed to update issue #{}", remote_id))?;
                    
                if !response.status().is_success() {
                    log::error!("   ⚠️ Update failed: {}", response.text().unwrap_or_default());
                }
            } else {
                // CREATE
                // log::info!("   Creating '{}'...", issue.title);
                
                let response = self.client.post(&url_base)
                    .header("PRIVATE-TOKEN", &token)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .context(format!("Failed to create issue: {}", issue.title))?;

                if response.status().is_success() {
                    let created: GitLabIssue = response.json()?;
                    issue.remotes.insert(self.config.provider.clone(), created.iid.to_string());
                } else {
                    log::error!("   ⚠️ Create failed: {}", response.text().unwrap_or_default());
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
        let remote_ids: std::collections::HashSet<String> = remote_issues.iter()
            .filter_map(|i| i.remotes.get(&self.config.provider).cloned())
            .collect();
        
        // Get local issue IIDs (for this provider)
        let local_ids: std::collections::HashSet<String> = local_issues.iter()
            .filter_map(|i| i.remotes.get(&self.config.provider).cloned())
            .collect();
        
        // IDs to delete = remote - local
        let to_delete: Vec<_> = remote_ids.difference(&local_ids).collect();
        
        let mut deleted = 0;
        for iid in to_delete {
            let url = format!("{}/{}", url_base, iid);
            // log::info!("   🗑️  Deleting remote issue #{}", iid);
            
            let response = self.client.delete(&url)
                .header("PRIVATE-TOKEN", &token)
                .send()
                .context(format!("Failed to delete issue #{}", iid))?;
                
            if response.status().is_success() {
                deleted += 1;
            } else {
                log::error!("   ⚠️ Delete failed: {}", response.text().unwrap_or_default());
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
        
        log::info!("🔀 Creating MR: {} -> {}", mr.source_branch, mr.target_branch);
        
        let response = self.client.post(&url)
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
        let iid = created["iid"].as_u64()
            .ok_or_else(|| anyhow!("No IID in response"))?;
        
        log::info!("✅ Created MR !{}", iid);
        Ok(iid)
    }
    
    fn list_mrs(&self) -> Result<Vec<crate::mr::MergeRequest>> {
        let token = self.get_token()?;
        let url = self.api_url("merge_requests?state=opened&per_page=100");
        
        let response = self.client.get(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to list merge requests")?;
            
        let gl_mrs: Vec<serde_json::Value> = response.json()
            .context("Failed to parse MR list")?;
            
        let mut mrs = Vec::new();
        for gl_mr in gl_mrs {
            let mr = crate::mr::MergeRequest {
                id: uuid::Uuid::new_v4().to_string(),
                remote_id: gl_mr["iid"].as_u64(),
                source_branch: gl_mr["source_branch"].as_str().unwrap_or_default().to_string(),
                target_branch: gl_mr["target_branch"].as_str().unwrap_or_default().to_string(),
                title: gl_mr["title"].as_str().unwrap_or_default().to_string(),
                description: gl_mr["description"].as_str().unwrap_or_default().to_string(),
                state: crate::mr::MRState::Open,  // Simplified for now
                author: gl_mr["author"]["username"].as_str().map(|s| s.to_string()),
                assignees: vec![],  // TODO: parse assignees
                labels: vec![],     // TODO: parse labels
                linked_issues: vec![],
                web_url: gl_mr["web_url"].as_str().map(|s| s.to_string()),
                created: chrono::Utc::now(), // TODO: parse timestamp
                updated: chrono::Utc::now(),
                merged_at: None,
                is_draft: gl_mr["draft"].as_bool().unwrap_or(false),
            };
            mrs.push(mr);
        }
        
        Ok(mrs)
    }
    
    fn get_mr(&self, remote_id: u64) -> Result<crate::mr::MergeRequest> {
        let token = self.get_token()?;
        let url = self.api_url(&format!("merge_requests/{}", remote_id));
        
        let response = self.client.get(&url)
            .header("PRIVATE-TOKEN", &token)
            .send()
            .context("Failed to get merge request")?;
            
        let gl_mr: serde_json::Value = response.json()?;
        
        Ok(crate::mr::MergeRequest {
            id: uuid::Uuid::new_v4().to_string(),
            remote_id: Some(remote_id),
            source_branch: gl_mr["source_branch"].as_str().unwrap_or_default().to_string(),
            target_branch: gl_mr["target_branch"].as_str().unwrap_or_default().to_string(),
            title: gl_mr["title"].as_str().unwrap_or_default().to_string(),
            description: gl_mr["description"].as_str().unwrap_or_default().to_string(),
            state: crate::mr::MRState::Open,
            author: gl_mr["author"]["username"].as_str().map(|s| s.to_string()),
            assignees: vec![],
            labels: vec![],
            linked_issues: vec![],
            web_url: gl_mr["web_url"].as_str().map(|s| s.to_string()),
            created: chrono::Utc::now(),
            updated: chrono::Utc::now(),
            merged_at: None,
            is_draft: gl_mr["draft"].as_bool().unwrap_or(false),
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
        
        let response = self.client.put(&url)
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
        // TODO: Implement GitLab MR approval
        Err(anyhow!("MR approval not yet implemented for GitLab"))
    }
    
    fn merge_mr(&self, remote_id: u64) -> Result<()> {
        // TODO: Implement GitLab MR merge
        Err(anyhow!("MR merge not yet implemented for GitLab"))
    }
    
    fn close_mr(&self, remote_id: u64) -> Result<()> {
        // TODO: Implement GitLab MR close
        Err(anyhow!("MR close not yet implemented for GitLab"))
    }
}
