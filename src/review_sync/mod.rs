// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Review-comment sync infrastructure shared between forge providers.
//!
//! Forgejo and GitLab disagree on what a "comment position" looks like:
//! Forgejo wants source-file line numbers, GitLab wants a triplet of
//! base/start/head SHAs plus old/new line. The shared work — verifying
//! that the comment's anchor `(file, line, commit)` actually exists at
//! the named commit — lives here. Provider-specific payload assembly
//! lives in `sync/forgejo.rs` and `sync/gitlab.rs`.
//!
//! [HAZMAT] Per Sprint C-heavy decision (4a), unresolvable positions
//! emit a `log::warn!` and are skipped — they do not abort the batch.
//! A whole-batch abort would punish other valid comments because of one
//! stale anchor (e.g. after the MR was rebased).

pub mod position;
