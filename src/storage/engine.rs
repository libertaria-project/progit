//! Storage Engine - JSON-only issue storage
//!
//! Single source of truth for issues. No KDL parsing for issues.

use crate::issue::Issue;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Storage engine for managing issues
pub struct StorageEngine {
    /// Path to issues.json
    issues_path: PathBuf,
    /// Loaded issues (cached in memory)
    issues: Vec<Issue>,
}

impl StorageEngine {
    /// Create a new storage engine for the given project root
    pub fn new(project_root: &Path) -> Self {
        let issues_path = project_root.join(".project").join("issues.json");
        Self {
            issues_path,
            issues: Vec::new(),
        }
    }

    /// Load all issues from disk
    pub fn load(&mut self) -> Result<&[Issue]> {
        if self.issues_path.exists() {
            let content = fs::read_to_string(&self.issues_path)
                .context("Failed to read issues.json")?;
            self.issues = serde_json::from_str(&content)
                .context("Failed to parse issues.json")?;
        } else {
            self.issues = Vec::new();
        }
        Ok(&self.issues)
    }

    /// Get all loaded issues
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Get mutable reference to issues
    pub fn issues_mut(&mut self) -> &mut Vec<Issue> {
        &mut self.issues
    }

    /// Save all issues to disk (atomic write)
    pub fn save(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.issues_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temp file first
        let tmp_path = self.issues_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(&self.issues)
            .context("Failed to serialize issues")?;
        fs::write(&tmp_path, &content)
            .context("Failed to write temp file")?;

        // Atomic rename
        fs::rename(&tmp_path, &self.issues_path)
            .context("Failed to rename temp file")?;

        Ok(())
    }

    /// Add or update an issue
    pub fn upsert(&mut self, issue: Issue) -> Result<()> {
        if let Some(existing) = self.issues.iter_mut().find(|i| i.id == issue.id) {
            *existing = issue;
        } else {
            self.issues.push(issue);
        }
        self.save()
    }

    /// Delete an issue by ID
    pub fn delete(&mut self, issue_id: &str) -> Result<bool> {
        let original_len = self.issues.len();
        self.issues.retain(|i| i.id != issue_id);
        
        if self.issues.len() < original_len {
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Find issue by ID
    pub fn find(&self, issue_id: &str) -> Option<&Issue> {
        self.issues.iter().find(|i| i.id == issue_id)
    }

    /// Find issue by ID (mutable)
    pub fn find_mut(&mut self, issue_id: &str) -> Option<&mut Issue> {
        self.issues.iter_mut().find(|i| i.id == issue_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_engine_roundtrip() {
        let dir = tempdir().unwrap();
        let mut engine = StorageEngine::new(dir.path());

        // Create and save
        let issue = Issue::new("Test Issue");
        let id = issue.id.clone();
        engine.upsert(issue).unwrap();

        // Reload
        let mut engine2 = StorageEngine::new(dir.path());
        engine2.load().unwrap();

        assert_eq!(engine2.issues().len(), 1);
        assert_eq!(engine2.issues()[0].id, id);
    }

    #[test]
    fn test_engine_delete() {
        let dir = tempdir().unwrap();
        let mut engine = StorageEngine::new(dir.path());

        let issue = Issue::new("Delete Me");
        let id = issue.id.clone();
        engine.upsert(issue).unwrap();

        assert!(engine.delete(&id).unwrap());
        assert_eq!(engine.issues().len(), 0);

        // Verify on disk
        let mut engine2 = StorageEngine::new(dir.path());
        engine2.load().unwrap();
        assert_eq!(engine2.issues().len(), 0);
    }
}
