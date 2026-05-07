// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Position resolver — verifies that a review comment's `(file, line,
//! commit)` anchor still exists at the named commit, before we ask a
//! forge to attach the comment there.
//!
//! [SEC] Doctrine-aligned skip-on-failure: a stale anchor (file
//! deleted, line shifted off end of file after rebase) is logged as a
//! warning, never crashes the host, and never blocks unrelated comments
//! from pushing.

use git2::Repository;
use std::path::Path;
use thiserror::Error;

/// Result of `resolve()` — enough for both Forgejo (line + head sha)
/// and GitLab (line + head sha; provider augments with diff_refs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPosition {
    pub file_path: String,
    pub line_number: u32,
    pub head_sha: String,
}

/// Errors that mean "skip this comment with a warning."
///
/// All variants are non-fatal at the batch level. The provider records
/// the warning, leaves the comment's `external_ids` alone (so it can be
/// retried on a future push when the anchor is once again valid), and
/// continues with the next comment.
#[derive(Debug, Error)]
pub enum PositionError {
    #[error("file '{0}' does not exist at commit {1}")]
    FileMissing(String, String),

    #[error("line {line} exceeds file '{file}' length ({total} lines) at commit {commit}")]
    LineOutOfRange {
        file: String,
        line: usize,
        total: usize,
        commit: String,
    },

    #[error("commit {0} not found in repository")]
    CommitMissing(String),

    #[error("git error: {0}")]
    Git(#[from] git2::Error),
}

/// Verify that `(file_path, line_number)` is a valid anchor at
/// `commit_sha`. Returns `Ok(ResolvedPosition)` on success.
///
/// Line numbers are 1-indexed (matching the convention in
/// `ReviewComment.line_number` and forge APIs).
pub fn resolve(
    repo: &Repository,
    file_path: &str,
    line_number: usize,
    commit_sha: &str,
) -> Result<ResolvedPosition, PositionError> {
    if line_number == 0 {
        return Err(PositionError::LineOutOfRange {
            file: file_path.to_string(),
            line: line_number,
            total: 0,
            commit: commit_sha.to_string(),
        });
    }

    let oid = repo
        .revparse_single(commit_sha)
        .map_err(|_| PositionError::CommitMissing(commit_sha.to_string()))?
        .id();
    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;

    let entry = match tree.get_path(Path::new(file_path)) {
        Ok(e) => e,
        Err(_) => {
            return Err(PositionError::FileMissing(
                file_path.to_string(),
                commit_sha.to_string(),
            ))
        }
    };

    let blob = entry.to_object(repo)?.peel_to_blob()?;
    let content = std::str::from_utf8(blob.content()).unwrap_or("");
    // count() walks all lines but we only need to know if line_number is
    // in range; for typical source files (<100K lines) the cost is
    // negligible. Avoid pre-allocating a Vec<&str>.
    let total_lines = content.lines().count();

    if line_number > total_lines {
        return Err(PositionError::LineOutOfRange {
            file: file_path.to_string(),
            line: line_number,
            total: total_lines,
            commit: commit_sha.to_string(),
        });
    }

    Ok(ResolvedPosition {
        file_path: file_path.to_string(),
        line_number: line_number as u32,
        head_sha: commit_sha.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, Signature};
    use std::fs;

    /// Build a tiny repo with one file at HEAD; return repo + commit SHA.
    fn make_test_repo(file_path: &str, content: &str) -> (tempfile::TempDir, Repository, String) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        let path = tmp.path().join(file_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();

        // Scope `tree` so its borrow on `repo` is released before we
        // move `repo` into the return tuple.
        let commit_oid = {
            let mut index = repo.index().unwrap();
            index
                .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
                .unwrap();
            index.write().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = Signature::now("test", "test@example.com").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "test", &tree, &[]).unwrap()
        };

        (tmp, repo, commit_oid.to_string())
    }

    #[test]
    fn resolves_valid_position() {
        let (_tmp, repo, sha) = make_test_repo("src/main.rs", "line1\nline2\nline3\n");
        let pos = resolve(&repo, "src/main.rs", 2, &sha).unwrap();
        assert_eq!(pos.file_path, "src/main.rs");
        assert_eq!(pos.line_number, 2);
        assert_eq!(pos.head_sha, sha);
    }

    #[test]
    fn rejects_missing_file() {
        let (_tmp, repo, sha) = make_test_repo("src/main.rs", "line1\n");
        let err = resolve(&repo, "src/never.rs", 1, &sha).unwrap_err();
        assert!(
            matches!(err, PositionError::FileMissing(ref f, _) if f == "src/never.rs"),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_line_out_of_range() {
        let (_tmp, repo, sha) = make_test_repo("src/main.rs", "line1\nline2\n");
        let err = resolve(&repo, "src/main.rs", 99, &sha).unwrap_err();
        match err {
            PositionError::LineOutOfRange { line, total, .. } => {
                assert_eq!(line, 99);
                assert_eq!(total, 2);
            }
            _ => panic!("expected LineOutOfRange, got {err:?}"),
        }
    }

    #[test]
    fn rejects_zero_line() {
        let (_tmp, repo, sha) = make_test_repo("src/main.rs", "x\n");
        let err = resolve(&repo, "src/main.rs", 0, &sha).unwrap_err();
        assert!(matches!(err, PositionError::LineOutOfRange { line: 0, .. }));
    }

    #[test]
    fn rejects_unknown_commit() {
        let (_tmp, repo, _sha) = make_test_repo("src/main.rs", "x\n");
        let err = resolve(&repo, "src/main.rs", 1, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef")
            .unwrap_err();
        assert!(matches!(err, PositionError::CommitMissing(_)), "got {err:?}");
    }
}
