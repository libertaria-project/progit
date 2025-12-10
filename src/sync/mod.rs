//! Sync Module - Bridge to the Cloud
//! 
//! Handles synchronization between local storage and remote forges.

pub mod forgejo;
pub mod gitlab;
pub mod keyring;

use anyhow::Result;
use crate::issue::Issue;
use crate::storage::config::SyncConfig;

/// Provider trait for remote issue trackers (Forgejo, GitLab, GitHub)
pub trait SyncProvider {
    fn login(&self) -> Result<()>;
    fn pull(&self) -> Result<Vec<Issue>>;
    fn push(&self, issues: &mut [Issue]) -> Result<()>;
    /// Delete remote issues not present in local set
    fn delete_missing(&self, local_issues: &[Issue]) -> Result<usize>;
}

pub fn create_provider(config: SyncConfig) -> Box<dyn SyncProvider> {
    match config.provider.as_str() {
        "forgejo" => Box::new(forgejo::ForgejoProvider::new(config)),
        "gitlab" => Box::new(gitlab::GitLabProvider::new(config)),
        _ => panic!("Unknown provider: {}", config.provider),
    }
}

/// Merge remote issues into local issues (timestamp-based merge)
/// Local changes are preserved if they're newer than remote
pub fn merge_issues(local_issues: &[Issue], remote_issues: Vec<Issue>, provider_name: &str) -> Vec<Issue> {
    let mut merged = local_issues.to_vec();
    
    for remote_issue in remote_issues {
        if let Some(remote_id) = remote_issue.remotes.get(provider_name) {
            // Check if we have this linked issue locally
            if let Some(existing_idx) = merged.iter().position(|i| i.remotes.get(provider_name) == Some(remote_id)) {
                // Timestamp-based merge: only update if remote is newer
                let local_updated = merged[existing_idx].updated;
                let local_title = merged[existing_idx].title.clone();
                
                if remote_issue.updated > local_updated {
                    // Remote is newer - update but preserve local ID and other remotes
                    println!("   ⬇ Updating local issue '{}' from remote #{} (remote newer)", local_title, remote_id);
                    let local_id = merged[existing_idx].id.clone();
                    let local_remotes = merged[existing_idx].remotes.clone();
                    
                    let mut updated = remote_issue.clone();
                    updated.id = local_id;
                    
                    // Preserve other remotes
                    for (k, v) in local_remotes.iter() {
                        if k != provider_name {
                            updated.remotes.insert(k.clone(), v.clone());
                        }
                    }
                    
                    merged[existing_idx] = updated;
                } else {
                    // Local is newer or equal - keep local, just ensure remote link exists
                    if !merged[existing_idx].remotes.contains_key(provider_name) {
                        merged[existing_idx].remotes.insert(provider_name.to_string(), remote_id.clone());
                    }
                    println!("   ⬆ Keeping local changes for '{}' (local newer or equal)", local_title);
                }
            } else if let Some(title_match_idx) = merged.iter().position(|i| i.title.trim().eq_ignore_ascii_case(remote_issue.title.trim())) {
                 // Match by Title (loose) - link them
                println!("   🔗 Linking local '{}' to remote #{} by title match", merged[title_match_idx].title, remote_id);
                 
                // Add remote link to local issue
                merged[title_match_idx].remotes.insert(provider_name.to_string(), remote_id.clone());
            } else {
                // Really new issue from remote
                println!("   ➕ Adding new remote issue: '{}'", remote_issue.title);
                merged.push(remote_issue);
            }
        } else {
            // Unlikely case: remote issue has no ID for its own provider
            merged.push(remote_issue);
        }
    }
    
    merged
}
