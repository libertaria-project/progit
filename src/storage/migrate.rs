//! Migration - Convert KDL issues to JSON
//!
//! One-time migration from the old KDL-based storage to JSON-only.

use crate::issue::Issue;
use crate::storage::kdl;
use crate::storage::engine::StorageEngine;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Migrate from KDL issues to JSON format
pub fn migrate_kdl_to_json(project_root: &Path) -> Result<usize> {
    let kdl_dir = project_root.join(".project").join("issues");
    let json_path = project_root.join(".project").join("issues.json");

    // Check if migration is needed
    if !kdl_dir.exists() {
        return Ok(0); // Nothing to migrate
    }

    // Check if already migrated
    if json_path.exists() {
        let kdl_count = count_kdl_files(&kdl_dir)?;
        if kdl_count == 0 {
            return Ok(0); // Already migrated
        }
        // Both exist - need to merge
        log::info!("Found both KDL and JSON issues - merging...");
    }

    // Read all KDL files
    let mut issues: Vec<Issue> = Vec::new();
    
    if kdl_dir.exists() && kdl_dir.is_dir() {
        for entry in fs::read_dir(&kdl_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "kdl") {
                match kdl::read_kdl(&path) {
                    Ok(issue) => {
                        log::info!("  Migrated: {} ({})", issue.title, issue.short_id());
                        issues.push(issue);
                    }
                    Err(e) => {
                        log::warn!("  Skipped {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    if issues.is_empty() {
        return Ok(0);
    }

    // Load existing JSON if present (merge)
    let mut engine = StorageEngine::new(project_root);
    if json_path.exists() {
        engine.load()?;
        // Merge: KDL issues take precedence (they are the source of truth)
        for kdl_issue in issues {
            engine.upsert(kdl_issue)?;
        }
    } else {
        // Fresh migration
        for issue in issues {
            engine.upsert(issue)?;
        }
    }

    let count = engine.issues().len();

    // Archive old KDL files (don't delete immediately)
    let archive_dir = project_root.join(".project").join("issues_kdl_backup");
    if count_kdl_files(&kdl_dir)? > 0 {
        fs::create_dir_all(&archive_dir)?;
        for entry in fs::read_dir(&kdl_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "kdl") {
                let dest = archive_dir.join(path.file_name().unwrap());
                fs::rename(&path, &dest)?;
            }
        }
        log::info!("📦 KDL files archived to {}", archive_dir.display());
    }

    Ok(count)
}

fn count_kdl_files(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let count = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "kdl"))
        .count();
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_migration_no_kdl() {
        let dir = tempdir().unwrap();
        let count = migrate_kdl_to_json(dir.path()).unwrap();
        assert_eq!(count, 0);
    }
}
