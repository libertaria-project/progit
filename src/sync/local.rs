use super::SyncProvider;
use crate::issue::Issue;
use crate::storage::config::SyncConfig;
use crate::mr::{MergeRequest, MRState};
use anyhow::{Result, Context};
use std::process::Command;

/// Local Provider Implementation
/// Treats local branches as "Merge Requests" against the main branch.
pub struct LocalProvider {
    config: SyncConfig,
    target_branch: String,
}

impl LocalProvider {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            target_branch: "main".to_string(), // Could be configurable
        }
    }

    fn run_git(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .output()
            .context("Failed to run git command")?;
            
        if !output.status.success() {
            return Err(anyhow::anyhow!("Git command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }
}

impl SyncProvider for LocalProvider {
    fn login(&self) -> Result<()> {
        // No login needed for local git
        Ok(())
    }

    fn pull(&self) -> Result<Vec<Issue>> {
        // Local provider doesn't pull issues from a remote
        // It relies on local file storage
        Ok(Vec::new())
    }

    fn push(&self, _issues: &mut [Issue]) -> Result<()> {
        // Local provider doesn't push issues
        Ok(())
    }

    fn delete_missing(&self, _local_issues: &[Issue]) -> Result<usize> {
        Ok(0)
    }

    // Merge Request Operations (Branch Management)
    
    fn create_mr(&self, mr: &MergeRequest) -> Result<u64> {
        // In local mode, creating an MR is essentially creating the branch if it doesn't exist
        // But usually, the branch exists first.
        // We'll just return a hash of the branch name as ID.
        // Or check if branch exists.
        
        let branch_exists = self.run_git(&["rev-parse", "--verify", &mr.source_branch]).is_ok();
        
        if !branch_exists {
            // Create branch? 
            self.run_git(&["checkout", "-b", &mr.source_branch])?;
        }
        
        // Return a pseudo-ID. We use a simple hash or just 0.
        // Since remote_id is u64, let's hash string.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        mr.source_branch.hash(&mut hasher);
        Ok(hasher.finish())
    }

    fn list_mrs(&self) -> Result<Vec<MergeRequest>> {
        // 1. Get all branches
        let output = self.run_git(&["for-each-ref", "--format=%(refname:short)", "refs/heads/"])?;
        
        let mut mrs = Vec::new();
        let branches: Vec<&str> = output.lines().collect();
        
        for branch in branches {
            if branch == self.target_branch {
                continue;
            }
            
            // 2. Check if ahead of main (has commits to merge)
            let ahead_check = self.run_git(&["rev-list", "--count", &format!("{}..{}", self.target_branch, branch)]);
            
            let commit_count = match ahead_check {
                Ok(count) => count.parse::<u64>().unwrap_or(0),
                Err(_) => 0, // Branch might be incompatible or error
            };
            
            if commit_count > 0 {
                // It's a valid "MR"
                let last_commit_msg = self.run_git(&["log", "-1", "--format=%s", branch]).unwrap_or_default();
                let author = self.run_git(&["log", "-1", "--format=%an", branch]).ok();
                
                // Determine state (merged?)
                // If it's fully merged, `git branch --merged main` would show it.
                // But if it has commits ahead, it's seemingly Open.
                
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                use std::hash::{Hash, Hasher};
                branch.hash(&mut hasher);
                let id = hasher.finish();

                mrs.push(MergeRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    remote_id: Some(id),
                    source_branch: branch.to_string(),
                    target_branch: self.target_branch.clone(),
                    title: if branch.contains('/') { 
                        // beautify feat/foo -> "Feat: Foo"
                        branch.split('/').last().unwrap_or(branch).to_string()
                    } else {
                        branch.to_string()
                    },
                    description: last_commit_msg,
                    state: MRState::Open,
                    author,
                    assignees: Vec::new(),
                    labels: Vec::new(),
                    linked_issues: Vec::new(),
                    web_url: None,
                    created: chrono::Utc::now(), // Estimate
                    updated: chrono::Utc::now(),
                    merged_at: None,
                    is_draft: branch.starts_with("draft/") || branch.starts_with("wip/"),
                });
            }
        }
        
        Ok(mrs)
    }

    fn get_mr(&self, remote_id: u64) -> Result<MergeRequest> {
        // Re-scan to find match. Inefficient but fine for local.
        let mrs = self.list_mrs()?;
        mrs.into_iter()
            .find(|mr| mr.remote_id == Some(remote_id))
            .ok_or_else(|| anyhow::anyhow!("Local MR not found for ID {}", remote_id))
    }

    fn update_mr(&self, _mr: &MergeRequest) -> Result<()> {
        // Local: Maybe rename branch? 
        // For now, no-op.
        Ok(())
    }
    
    fn approve_mr(&self, _remote_id: u64) -> Result<()> {
        // Local mode: no approval needed
        Ok(())
    }
    
    fn merge_mr(&self, remote_id: u64) -> Result<()> {
        // Find the branch for this MR
        let mr = self.get_mr(remote_id)?;
        
        // Merge the branch into target
        self.run_git(&["checkout", &self.target_branch])?;
        self.run_git(&["merge", "--no-ff", &mr.source_branch])?;
        
        Ok(())
    }
    
    fn close_mr(&self, remote_id: u64) -> Result<()> {
        // Find the branch for this MR
        let mr = self.get_mr(remote_id)?;
        
        // Delete the branch (close without merging)
        self.run_git(&["branch", "-D", &mr.source_branch])?;
        
        Ok(())
    }
}
