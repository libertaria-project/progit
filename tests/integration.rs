// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2026 Markus Maiwald
//
// Integration tests: critical path — init, issue lifecycle, virtual branches, config

use anyhow::Result;
use progit::issue::{Issue, Status};
use progit::storage::{delete_issue, load_issues, save_issue, sync_kdl_to_json};
use std::fs;
use tempfile::tempdir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_project_dir(base: &std::path::Path) -> std::path::PathBuf {
    let issues_dir = base.join(".project").join("issues");
    fs::create_dir_all(&issues_dir).unwrap();
    issues_dir
}

fn cache_path(base: &std::path::Path) -> std::path::PathBuf {
    base.join(".project").join("issues.json")
}

fn new_issue(title: &str) -> Issue {
    Issue::new(title.to_string())
}

// ─── 1. Init → create → save → load ──────────────────────────────────────────

#[test]
fn test_create_save_and_reload_issue() -> Result<()> {
    let dir = tempdir()?;
    let kdl_dir = make_project_dir(dir.path());
    let cache = cache_path(dir.path());

    let issue = new_issue("Add authentication module");
    let id = issue.id.clone();

    // Save
    save_issue(&issue, &kdl_dir, &cache)?;

    // Reload from disk
    let loaded = load_issues(&kdl_dir, &cache)?;
    assert_eq!(loaded.len(), 1, "Exactly one issue should be loaded");
    assert_eq!(loaded[0].id, id);
    assert_eq!(loaded[0].title, "Add authentication module");
    assert_eq!(loaded[0].status, Status::Backlog);

    Ok(())
}

// ─── 2. Update status → save → reload ────────────────────────────────────────

#[test]
fn test_update_status_persists() -> Result<()> {
    let dir = tempdir()?;
    let kdl_dir = make_project_dir(dir.path());
    let cache = cache_path(dir.path());

    let mut issue = new_issue("Fix null pointer in parser");
    save_issue(&issue, &kdl_dir, &cache)?;

    // Mutate and re-save
    issue.status = Status::InProgress;
    save_issue(&issue, &kdl_dir, &cache)?;

    let loaded = load_issues(&kdl_dir, &cache)?;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].status, Status::InProgress);

    Ok(())
}

// ─── 3. Multi-issue create → delete → verify count ───────────────────────────

#[test]
fn test_multi_issue_create_and_delete() -> Result<()> {
    let dir = tempdir()?;
    let kdl_dir = make_project_dir(dir.path());
    let cache = cache_path(dir.path());

    let a = new_issue("Issue Alpha");
    let b = new_issue("Issue Beta");
    let c = new_issue("Issue Gamma");

    let id_b = b.id.clone();

    save_issue(&a, &kdl_dir, &cache)?;
    save_issue(&b, &kdl_dir, &cache)?;
    save_issue(&c, &kdl_dir, &cache)?;

    let before = load_issues(&kdl_dir, &cache)?;
    assert_eq!(before.len(), 3);

    // Delete B
    let deleted = delete_issue(&id_b, &kdl_dir, &cache)?;
    assert!(deleted, "delete_issue should return true for existing issue");

    let after = load_issues(&kdl_dir, &cache)?;
    assert_eq!(after.len(), 2);
    assert!(!after.iter().any(|i| i.id == id_b), "Deleted issue must not appear in reload");

    Ok(())
}

// ─── 4. Delete non-existent issue returns false ───────────────────────────────

#[test]
fn test_delete_nonexistent_issue_returns_false() -> Result<()> {
    let dir = tempdir()?;
    let kdl_dir = make_project_dir(dir.path());
    let cache = cache_path(dir.path());

    let deleted = delete_issue("does-not-exist", &kdl_dir, &cache)?;
    assert!(!deleted, "Deleting non-existent issue should return false, not panic");

    Ok(())
}

// ─── 5. KDL → JSON cache sync is idempotent ───────────────────────────────────

#[test]
fn test_kdl_to_json_sync_is_idempotent() -> Result<()> {
    let dir = tempdir()?;
    let kdl_dir = make_project_dir(dir.path());
    let cache = cache_path(dir.path());

    let issue = new_issue("Cache sync test");
    save_issue(&issue, &kdl_dir, &cache)?;

    // Sync twice
    let first = sync_kdl_to_json(&kdl_dir, &cache)?;
    let second = sync_kdl_to_json(&kdl_dir, &cache)?;

    assert_eq!(first.len(), second.len(), "Idempotent sync should return same count");
    assert_eq!(first[0].id, second[0].id);

    Ok(())
}

// ─── 6. Issue with tags and assignee round-trips cleanly ─────────────────────

#[test]
fn test_issue_metadata_round_trip() -> Result<()> {
    let dir = tempdir()?;
    let kdl_dir = make_project_dir(dir.path());
    let cache = cache_path(dir.path());

    let mut issue = new_issue("Metadata round-trip");
    issue.tags = vec!["backend".to_string(), "urgent".to_string()];
    issue.assignee = Some("markus".to_string());
    issue.status = Status::Done;

    save_issue(&issue, &kdl_dir, &cache)?;

    let loaded = load_issues(&kdl_dir, &cache)?;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].tags, vec!["backend", "urgent"]);
    assert_eq!(loaded[0].assignee.as_deref(), Some("markus"));
    assert_eq!(loaded[0].status, Status::Done);

    Ok(())
}
