use anyhow::Result;
use prog::issue::{Issue, Status};
use prog::storage::{delete_issue, parse_kdl, read_kdl, save_issue, sync_kdl_to_json};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_ghost_issue_persistence_and_deletion() -> Result<()> {
    // Setup a temp directory simulating the project structure
    let dir = tempdir()?;
    let kdl_dir = dir.path().join("issues");
    let cache_path = dir.path().join("issues.json"); // Cache not strictly needed but expected by APIs

    fs::create_dir_all(&kdl_dir)?;

    // 1. Create a "Ghost" KDL file on disk (NO ID)
    // This simulates the "default configs" or manually added files that lack IDs.
    let ghost_filename = "ghost-issue.kdl";
    let ghost_path = kdl_dir.join(ghost_filename);
    let ghost_content = r#"
issue {
    title "I am a ghost"
    status "backlog"
}
"#;
    fs::write(&ghost_path, ghost_content)?;

    // 2. Read the issue using `read_kdl`
    // This should trigger the new logic: generate ID AND write it back to disk.
    let issue = read_kdl(&ghost_path)?;
    println!("Read issue with generated ID: {}", issue.id);

    assert!(!issue.id.is_empty(), "Issue should have a generated ID");

    // 3. Verify the ID was written back to the file
    let content_after = fs::read_to_string(&ghost_path)?;
    assert!(content_after.contains(&format!("id=\"{}\"", issue.id)), "File should now contain the generated ID");

    // 4. Try to delete the issue using the generated ID
    // `delete_issue` scans the directory. It reads files.
    // If our fix works, it reads `ghost-issue.kdl`, sees the MATCHING ID (because it was persisted), and deletes it.
    let deleted = delete_issue(&issue.id, &kdl_dir, &cache_path)?;
    
    assert!(deleted, "delete_issue should return true");
    assert!(!ghost_path.exists(), "Ghost file should be deleted");

    Ok(())
}
