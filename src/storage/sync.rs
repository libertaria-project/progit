//! Sync - KDL to JSON synchronization
//!
//! Keeps the JSON cache in sync with KDL source files.

use super::{json, kdl};
use crate::issue::Issue;
use anyhow::Result;
use std::path::Path;

/// Sync KDL files to JSON cache
pub fn sync_kdl_to_json(kdl_dir: &Path, cache_path: &Path) -> Result<Vec<Issue>> {
    // Read all KDL files
    let issues = kdl::read_all_kdl(kdl_dir)?;

    // Write to JSON cache
    json::write_cache(&issues, cache_path)?;

    Ok(issues)
}

/// Load issues fresh from KDL (always re-sync cache)
/// This ensures manual edits to KDL files are picked up
pub fn load_issues(kdl_dir: &Path, cache_path: &Path) -> Result<Vec<Issue>> {
    // Always read from KDL source of truth and update cache
    sync_kdl_to_json(kdl_dir, cache_path)
}

/// Save a single issue (writes KDL, updates cache)
pub fn save_issue(issue: &Issue, kdl_dir: &Path, cache_path: &Path) -> Result<()> {
    let new_filename = kdl::issue_filename(issue);
    let new_path = kdl_dir.join(&new_filename);

    // Clean up any old files for this issue (renames/duplicates)
    // We do this BEFORE writing the new one to avoid deleting what we just wrote
    // if the filenames happen to collide in some weird way (unlikely)
    // or to ensure we don't leave ghosts.
    if kdl_dir.exists() {
        for entry in std::fs::read_dir(kdl_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip non-kdl files
            if path.extension().map_or(true, |ext| ext != "kdl") {
                continue;
            }

            // Skip if it's exactly the file we are about to write
            if path.file_name() == Some(std::ffi::OsStr::new(&new_filename)) {
                continue;
            }

            // Read to check ID
            if let Ok(existing) = kdl::read_kdl(&path) {
                if existing.id == issue.id {
                    // Found an old file for this ID with a different name -> Delete it
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }

    // Write KDL file
    kdl::write_kdl(issue, &new_path)?;

    // Reload and update cache
    sync_kdl_to_json(kdl_dir, cache_path)?;

    Ok(())
}

/// Delete an issue
pub fn delete_issue(issue_id: &str, kdl_dir: &Path, cache_path: &Path) -> Result<bool> {
    let mut deleted = false;

    // Find and delete the KDL file(s)
    if kdl_dir.exists() {
        for entry in std::fs::read_dir(kdl_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "kdl") {
                if let Ok(issue) = kdl::read_kdl(&path) {
                    if issue.id == issue_id {
                        std::fs::remove_file(&path)?;
                        deleted = true;
                        // Continue loop to clean up any duplicates
                    }
                }
            }
        }
    }

    if deleted {
        // Resync cache
        sync_kdl_to_json(kdl_dir, cache_path)?;
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Status;
    use tempfile::tempdir;

    #[test]
    fn test_sync_roundtrip() {
        let dir = tempdir().unwrap();
        let kdl_dir = dir.path().join("issues");
        let cache_path = dir.path().join(".cache/issues.json");

        std::fs::create_dir_all(&kdl_dir).unwrap();

        // Create a KDL file manually
        let kdl_content = r#"
issue id="test-001" {
    title "Test sync"
    status "backlog"
    effort 3
}
"#;
        std::fs::write(kdl_dir.join("test-001.kdl"), kdl_content).unwrap();

        // Sync
        let issues = sync_kdl_to_json(&kdl_dir, &cache_path).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "Test sync");

        // Load from cache
        let loaded = load_issues(&kdl_dir, &cache_path).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_save_and_delete() {
        let dir = tempdir().unwrap();
        let kdl_dir = dir.path().join("issues");
        let cache_path = dir.path().join(".cache/issues.json");

        let issue = Issue::new("Save me").with_status(Status::InProgress);
        let issue_id = issue.id.clone();

        // Save
        save_issue(&issue, &kdl_dir, &cache_path).unwrap();

        // Verify
        let issues = load_issues(&kdl_dir, &cache_path).unwrap();
        assert_eq!(issues.len(), 1);

        // Delete
        let deleted = delete_issue(&issue_id, &kdl_dir, &cache_path).unwrap();
        assert!(deleted);

        // Verify deleted
        let issues = load_issues(&kdl_dir, &cache_path).unwrap();
        assert!(issues.is_empty());
    }
}
