// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Contract test for `SyncProvider::push_review_comments` idempotency.
//!
//! Verifies the *trait contract* — any provider whose impl follows the
//! "skip if external_ids[<provider>] present, insert after success" rule
//! will be idempotent. The Forgejo and GitLab implementations both follow
//! this rule; this mock proves the rule itself is correct.

#![cfg(test)]

use crate::issue::Issue;
use crate::mr::MergeRequest;
use crate::review::{Review, ReviewComment, ReviewStatus};
use crate::sync::SyncProvider;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// Minimal SyncProvider that mints predictable external IDs for the
/// purpose of validating the idempotency contract. All non-relevant
/// methods are `unimplemented!()` — never invoked by the test.
struct MockProvider {
    name: &'static str,
}

impl SyncProvider for MockProvider {
    fn login(&self) -> Result<()> {
        unimplemented!()
    }
    fn pull(&self) -> Result<Vec<Issue>> {
        unimplemented!()
    }
    fn push(&self, _issues: &mut [Issue]) -> Result<()> {
        unimplemented!()
    }
    fn delete_missing(&self, _local: &[Issue]) -> Result<usize> {
        unimplemented!()
    }
    fn create_mr(&self, _mr: &MergeRequest) -> Result<u64> {
        unimplemented!()
    }
    fn list_mrs(&self) -> Result<Vec<MergeRequest>> {
        unimplemented!()
    }
    fn get_mr(&self, _remote_id: u64) -> Result<MergeRequest> {
        unimplemented!()
    }
    fn update_mr(&self, _mr: &MergeRequest) -> Result<()> {
        unimplemented!()
    }
    fn approve_mr(&self, _remote_id: u64) -> Result<()> {
        unimplemented!()
    }
    fn merge_mr(&self, _remote_id: u64) -> Result<()> {
        unimplemented!()
    }
    fn close_mr(&self, _remote_id: u64) -> Result<()> {
        unimplemented!()
    }

    fn push_review_comments(
        &self,
        _repo_path: &Path,
        _mr_remote_id: u64,
        _review: &Review,
        comments: &mut [ReviewComment],
    ) -> Result<usize> {
        let mut count = 0;
        for c in comments.iter_mut() {
            if c.external_ids.contains_key(self.name) {
                continue; // already synced — skip
            }
            c.external_ids
                .insert(self.name.to_string(), format!("remote-{}", c.id));
            count += 1;
        }
        Ok(count)
    }
}

fn comment(id: &str) -> ReviewComment {
    ReviewComment {
        id: id.into(),
        file_path: "x.rs".into(),
        line_number: 1,
        commit_sha: "deadbeef".into(),
        text: "n/a".into(),
        author: "tester".into(),
        created_at: "2026-05-07T00:00:00Z".into(),
        resolved: false,
        replies: vec![],
        external_ids: HashMap::new(),
    }
}

fn empty_review() -> Review {
    Review {
        id: "r1".into(),
        mr_id: None,
        commit_sha: "deadbeef".into(),
        status: ReviewStatus::InProgress,
        reviewer: "tester".into(),
        verdict: None,
        summary: None,
        comments: vec![],
        created_at: "2026-05-07T00:00:00Z".into(),
        updated_at: "2026-05-07T00:00:00Z".into(),
    }
}

#[test]
fn idempotent_push_skips_already_synced_comments() {
    let provider = MockProvider { name: "mock" };
    let review = empty_review();
    let path = Path::new("/tmp");
    let mut comments = vec![comment("c1"), comment("c2")];

    // First push: both new → count == 2
    let n = provider
        .push_review_comments(path, 42, &review, &mut comments)
        .unwrap();
    assert_eq!(n, 2);
    assert_eq!(
        comments[0].external_ids.get("mock"),
        Some(&"remote-c1".to_string())
    );
    assert_eq!(
        comments[1].external_ids.get("mock"),
        Some(&"remote-c2".to_string())
    );

    // Second push of the same slice: no-op
    let n2 = provider
        .push_review_comments(path, 42, &review, &mut comments)
        .unwrap();
    assert_eq!(n2, 0, "idempotency: second push must be a no-op");

    // Third push after appending a new comment: only the new one syncs.
    comments.push(comment("c3"));
    let n3 = provider
        .push_review_comments(path, 42, &review, &mut comments)
        .unwrap();
    assert_eq!(n3, 1, "only the newly-added comment should sync");
    assert_eq!(
        comments[2].external_ids.get("mock"),
        Some(&"remote-c3".to_string())
    );
}

#[test]
fn provider_keys_do_not_clobber_each_other() {
    // A comment synced to Forgejo can still be pushed to GitLab — the
    // two provider entries coexist in external_ids.
    let forgejo = MockProvider { name: "forgejo" };
    let gitlab = MockProvider { name: "gitlab" };
    let review = empty_review();
    let path = Path::new("/tmp");
    let mut comments = vec![comment("c1")];

    forgejo
        .push_review_comments(path, 1, &review, &mut comments)
        .unwrap();
    gitlab
        .push_review_comments(path, 1, &review, &mut comments)
        .unwrap();

    let ids = &comments[0].external_ids;
    assert_eq!(ids.get("forgejo"), Some(&"remote-c1".to_string()));
    assert_eq!(ids.get("gitlab"), Some(&"remote-c1".to_string()));

    // Re-pushing to either is still a no-op.
    let nf = forgejo
        .push_review_comments(path, 1, &review, &mut comments)
        .unwrap();
    let ng = gitlab
        .push_review_comments(path, 1, &review, &mut comments)
        .unwrap();
    assert_eq!(nf, 0);
    assert_eq!(ng, 0);
}
