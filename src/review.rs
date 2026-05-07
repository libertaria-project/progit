// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Code review system
//!
//! Line-level comments on diffs for collaborative code review.
//! Stores reviews in .project/reviews/ as JSON files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A code review comment on a specific line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    /// Unique comment ID (local UUID).
    pub id: String,

    /// File path relative to repo root
    pub file_path: String,

    /// Line number (1-indexed)
    pub line_number: usize,

    /// Commit SHA this comment is attached to
    pub commit_sha: String,

    /// Comment text
    pub text: String,

    /// Author username
    pub author: String,

    /// Timestamp (ISO 8601)
    pub created_at: String,

    /// Resolved status — local UX hint only. Forge sync ignores this field
    /// (option (b) from Sprint C-heavy: push all comments including resolved).
    pub resolved: bool,

    /// Optional thread (replies to this comment)
    #[serde(default)]
    pub replies: Vec<ReviewComment>,

    /// Per-provider external comment IDs after a successful forge push.
    ///
    /// Keyed by provider name (`"forgejo"`, `"gitlab"`). The presence of
    /// a key means the comment has been synced to that provider; sync
    /// is idempotent — re-pushing skips comments that already have an
    /// entry. Default empty so v0.1 reviews migrate forward losslessly.
    #[serde(default)]
    pub external_ids: HashMap<String, String>,
}

/// A code review session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Review ID (UUID or commit SHA)
    pub id: String,

    /// Merge request ID (if applicable)
    pub mr_id: Option<String>,

    /// Commit SHA being reviewed
    pub commit_sha: String,

    /// Review status
    pub status: ReviewStatus,

    /// Reviewer username
    pub reviewer: String,

    /// Overall verdict
    pub verdict: Option<ReviewVerdict>,

    /// Summary comment
    pub summary: Option<String>,

    /// Line-level comments
    #[serde(default)]
    pub comments: Vec<ReviewComment>,

    /// Created timestamp
    pub created_at: String,

    /// Updated timestamp
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewVerdict {
    Approve,
    RequestChanges,
    Comment,
}

/// Review storage manager
pub struct ReviewStorage {
    reviews_dir: PathBuf,
}

impl ReviewStorage {
    /// Create new review storage
    pub fn new(project_root: &Path) -> Self {
        let reviews_dir = project_root.join(".project").join("reviews");
        Self { reviews_dir }
    }

    /// Initialize storage directory
    pub fn init(&self) -> Result<()> {
        if !self.reviews_dir.exists() {
            fs::create_dir_all(&self.reviews_dir)
                .context("Failed to create reviews directory")?;
        }
        Ok(())
    }

    /// Save a review
    pub fn save(&self, review: &Review) -> Result<()> {
        self.init()?;

        let file_path = self.reviews_dir.join(format!("{}.json", review.id));
        let json = serde_json::to_string_pretty(review)
            .context("Failed to serialize review")?;

        fs::write(&file_path, json)
            .with_context(|| format!("Failed to write review: {}", file_path.display()))?;

        Ok(())
    }

    /// Load a review by ID
    pub fn load(&self, review_id: &str) -> Result<Review> {
        let file_path = self.reviews_dir.join(format!("{}.json", review_id));
        let json = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read review: {}", file_path.display()))?;

        let review: Review = serde_json::from_str(&json)
            .context("Failed to deserialize review")?;

        Ok(review)
    }

    /// List all reviews
    pub fn list(&self) -> Result<Vec<Review>> {
        if !self.reviews_dir.exists() {
            return Ok(Vec::new());
        }

        let mut reviews = Vec::new();

        for entry in fs::read_dir(&self.reviews_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(review) = self.load(
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                ) {
                    reviews.push(review);
                }
            }
        }

        Ok(reviews)
    }

    /// Get comments for a specific file and commit
    pub fn get_comments_for_file(
        &self,
        file_path: &str,
        commit_sha: &str,
    ) -> Result<Vec<ReviewComment>> {
        let reviews = self.list()?;

        let mut comments = Vec::new();
        for review in reviews {
            if review.commit_sha == commit_sha {
                for comment in review.comments {
                    if comment.file_path == file_path {
                        comments.push(comment);
                    }
                }
            }
        }

        Ok(comments)
    }

    /// Add a comment to a review
    pub fn add_comment(
        &self,
        review_id: &str,
        comment: ReviewComment,
    ) -> Result<()> {
        let mut review = self.load(review_id)?;
        review.comments.push(comment);
        review.updated_at = chrono::Utc::now().to_rfc3339();
        self.save(&review)?;
        Ok(())
    }

    /// Update review status
    pub fn update_status(
        &self,
        review_id: &str,
        status: ReviewStatus,
        verdict: Option<ReviewVerdict>,
    ) -> Result<()> {
        let mut review = self.load(review_id)?;
        review.status = status;
        review.verdict = verdict;
        review.updated_at = chrono::Utc::now().to_rfc3339();
        self.save(&review)?;
        Ok(())
    }
}

/// Group comments by file and line number
pub fn group_comments_by_line(
    comments: Vec<ReviewComment>,
) -> HashMap<String, HashMap<usize, Vec<ReviewComment>>> {
    let mut grouped: HashMap<String, HashMap<usize, Vec<ReviewComment>>> = HashMap::new();

    for comment in comments {
        grouped
            .entry(comment.file_path.clone())
            .or_default()
            .entry(comment.line_number)
            .or_default()
            .push(comment);
    }

    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = ReviewStorage::new(temp_dir.path());

        let review = Review {
            id: "test-review".to_string(),
            mr_id: None,
            commit_sha: "abc123".to_string(),
            status: ReviewStatus::InProgress,
            reviewer: "alice".to_string(),
            verdict: None,
            summary: None,
            comments: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        storage.save(&review).unwrap();

        let loaded = storage.load("test-review").unwrap();
        assert_eq!(loaded.id, "test-review");
        assert_eq!(loaded.commit_sha, "abc123");
    }

    #[test]
    fn test_add_comment() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = ReviewStorage::new(temp_dir.path());

        let review = Review {
            id: "test-review".to_string(),
            mr_id: None,
            commit_sha: "abc123".to_string(),
            status: ReviewStatus::InProgress,
            reviewer: "alice".to_string(),
            verdict: None,
            summary: None,
            comments: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        storage.save(&review).unwrap();

        let comment = ReviewComment {
            id: "comment-1".to_string(),
            file_path: "src/main.rs".to_string(),
            line_number: 42,
            commit_sha: "abc123".to_string(),
            text: "Consider using Result<T, E> here".to_string(),
            author: "bob".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            resolved: false,
            replies: vec![],
            external_ids: HashMap::new(),
        };

        storage.add_comment("test-review", comment).unwrap();

        let loaded = storage.load("test-review").unwrap();
        assert_eq!(loaded.comments.len(), 1);
        assert_eq!(loaded.comments[0].line_number, 42);
        assert!(loaded.comments[0].external_ids.is_empty());
    }

    #[test]
    fn loads_v01_review_without_external_ids_field() {
        // Pre-Sprint-C reviews don't have `external_ids` in their JSON.
        // Serde must default to an empty map — losslessly migrating forward.
        let json = r#"{
            "id": "old-review",
            "mr_id": null,
            "commit_sha": "deadbeef",
            "status": "inprogress",
            "reviewer": "alice",
            "verdict": null,
            "summary": null,
            "comments": [{
                "id": "c1",
                "file_path": "src/x.rs",
                "line_number": 7,
                "commit_sha": "deadbeef",
                "text": "nit",
                "author": "alice",
                "created_at": "2026-01-01T00:00:00Z",
                "resolved": false
            }],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }"#;
        let r: Review = serde_json::from_str(json).expect("v0.1 review must still parse");
        assert_eq!(r.comments.len(), 1);
        assert!(r.comments[0].external_ids.is_empty());
        assert!(r.comments[0].replies.is_empty());
    }
}
