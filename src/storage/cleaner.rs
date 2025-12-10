use crate::issue::{Issue, Status};
use crate::storage::{self, paths};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Clean up duplicate issues by merging them
pub fn cleanup_duplicates(root: &std::path::Path) -> Result<()> {
    let kdl_dir = root.join(paths::PROJECT_DIR).join("issues");
    let cache_path = root.join(paths::LOCAL_DIR).join("issues.json");

    let issues = storage::load_issues(&kdl_dir, &cache_path)?;
    println!("🧹 Analyzing {} issues for duplicates...", issues.len());

    // Group by Title
    let mut groups: HashMap<String, Vec<Issue>> = HashMap::new();
    for issue in issues {
        groups.entry(issue.title.trim().to_string()) // Trim to catch "Foo " vs "Foo"
            .or_default()
            .push(issue);
    }

    let mut issues_to_save = Vec::new();
    let mut issues_to_delete = Vec::new();

    for (title, mut group) in groups {
        if group.len() > 1 {
            println!("   Duplicate Group: '{}' ({} found)", title, group.len());
            
            // Sort by creation date (Oldest first will be master)
            group.sort_by_key(|i| i.created);
            
            let mut master = group[0].clone();
            
            // Merge others into master
            for duplicate in group.iter().skip(1) {
                // Merge Status (Take the most advanced)
                if status_rank(duplicate.status) > status_rank(master.status) {
                    master.status = duplicate.status;
                }
                
                // Merge Tags
                let mut tags: HashSet<String> = master.tags.iter().cloned().collect();
                tags.extend(duplicate.tags.iter().cloned());
                master.tags = tags.into_iter().collect();
                
                // Merge Remotes
                for (provider, id) in &duplicate.remotes {
                    if !master.remotes.contains_key(provider) {
                        master.remotes.insert(provider.clone(), id.clone());
                    }
                }
                
                // Mark for deletion
                issues_to_delete.push(duplicate.id.clone());
            }
            
            issues_to_save.push(master);
        } else {
            issues_to_save.push(group[0].clone());
        }
    }
    
    // Execute Changes
    if !issues_to_delete.is_empty() {
        println!("✨ Merging duplicates...");
        for issue in &issues_to_save {
            storage::save_issue(issue, &kdl_dir, &cache_path)?;
        }
        
        println!("🗑️  Cleaning up clones...");
        for id in issues_to_delete {
            storage::delete_issue(&id, &kdl_dir, &cache_path)?;
        }
        
        // Force sync json
        storage::sync_kdl_to_json(&kdl_dir, &cache_path)?;
        println!("✅ Cleanup complete. Saved {} issues.", issues_to_save.len());
    } else {
        println!("✅ No duplicates found.");
    }

    Ok(())
}

fn status_rank(status: Status) -> u8 {
    match status {
        Status::Done => 3,
        Status::InProgress => 2,
        Status::Backlog => 1,
    }
}
