//! Storage Engine - JSON-only issue storage
//!
//! Single source of truth for issues. No KDL parsing for issues.

use crate::issue::Issue;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Storage engine for managing issues and merge requests
pub struct StorageEngine {
    /// Path to issues.json
    issues_path: PathBuf,
    /// Path to mrs.json
    mrs_path: PathBuf,
    /// Loaded issues (cached in memory)
    issues: Vec<Issue>,
    /// Loaded merge requests (cached in memory)
    mrs: Vec<crate::mr::MergeRequest>,
}

impl StorageEngine {
    /// Create a new storage engine for the given project root
    pub fn new(project_root: &Path) -> Self {
        let issues_path = project_root.join(".project").join("issues.json");
        let mrs_path = project_root.join(".project").join("mrs.json");
        Self {
            issues_path,
            mrs_path,
            issues: Vec::new(),
            mrs: Vec::new(),
        }
    }

    /// Load all data from disk (supporting both single file and directory mode)
    pub fn load(&mut self) -> Result<()> {
        self.issues.clear();
        self.mrs.clear();

        // 1. Load Issues
        if self.issues_path.exists() {
            let content =
                fs::read_to_string(&self.issues_path).context("Failed to read issues.json")?;
            if let Ok(issues) = serde_json::from_str::<Vec<Issue>>(&content) {
                self.issues = issues;
            }
        }

        let issues_dir = self.issues_path.parent().unwrap().join("issues");
        for path in regular_json_files(&issues_dir)? {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(issue) = serde_json::from_str::<Issue>(&content) {
                    if let Some(existing) = self.issues.iter_mut().find(|i| i.id == issue.id) {
                        *existing = issue;
                    } else {
                        self.issues.push(issue);
                    }
                }
            }
        }

        // 2. Load Merge Requests
        if self.mrs_path.exists() {
            let content = fs::read_to_string(&self.mrs_path).context("Failed to read mrs.json")?;
            if let Ok(mrs) = serde_json::from_str::<Vec<crate::mr::MergeRequest>>(&content) {
                self.mrs = mrs;
            }
        }

        let mrs_dir = self.mrs_path.parent().unwrap().join("mrs");
        for path in regular_json_files(&mrs_dir)? {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mr) = serde_json::from_str::<crate::mr::MergeRequest>(&content) {
                    if let Some(existing) = self.mrs.iter_mut().find(|m| m.id == mr.id) {
                        *existing = mr;
                    } else {
                        self.mrs.push(mr);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get all loaded issues
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Get all loaded merge requests
    pub fn mrs(&self) -> &[crate::mr::MergeRequest] {
        &self.mrs
    }

    /// Get mutable reference to issues
    pub fn issues_mut(&mut self) -> &mut Vec<Issue> {
        &mut self.issues
    }

    /// Get mutable reference to merge requests
    pub fn mrs_mut(&mut self) -> &mut Vec<crate::mr::MergeRequest> {
        &mut self.mrs
    }

    /// Save everything to disk (atomic write)
    pub fn save(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.issues_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 1. Save Issues
        let tmp_issues = self.issues_path.with_extension("json.tmp");
        let issues_content =
            serde_json::to_string_pretty(&self.issues).context("Failed to serialize issues")?;
        fs::write(&tmp_issues, &issues_content)?;
        fs::rename(&tmp_issues, &self.issues_path)?;

        // 2. Save Merge Requests
        let tmp_mrs = self.mrs_path.with_extension("json.tmp");
        let mrs_content =
            serde_json::to_string_pretty(&self.mrs).context("Failed to serialize mrs")?;
        fs::write(&tmp_mrs, &mrs_content)?;
        fs::rename(&tmp_mrs, &self.mrs_path)?;

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

    /// Add or update an MR
    pub fn upsert_mr(&mut self, mr: crate::mr::MergeRequest) -> Result<()> {
        if let Some(existing) = self.mrs.iter_mut().find(|m| m.id == mr.id) {
            *existing = mr;
        } else {
            self.mrs.push(mr);
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

fn regular_json_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("Failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let file_type = entry
                .file_type()
                .with_context(|| format!("Failed to inspect {}", path.display()))?;
            if !file_type.is_file() {
                bail!("{} must be a regular JSON file", path.display());
            }
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
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

    #[cfg(unix)]
    #[test]
    fn test_engine_rejects_symlinked_issue_file() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let issues_dir = dir.path().join(".project/issues");
        fs::create_dir_all(&issues_dir).unwrap();
        let outside = dir.path().join("outside.json");
        fs::write(
            &outside,
            serde_json::to_string(&Issue::new("Outside")).unwrap(),
        )
        .unwrap();
        symlink(&outside, issues_dir.join("link.json")).unwrap();

        let mut engine = StorageEngine::new(dir.path());
        let err = engine.load().unwrap_err().to_string();

        assert!(err.contains("regular JSON file"));
    }
}
