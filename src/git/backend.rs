// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2026 Markus Maiwald

//! # GitBackend integration for the ProGit TUI
//!
//! Two backends ship behind the `forge-backend` feature:
//!
//! - [`LocalGitBackend`] — operates on a local-disk git repo via `gix`.
//!   Useful for users who do not run a `progit-forged` daemon and want
//!   the TUI's git data plane to operate purely against the filesystem.
//!
//! - [`ForgedBackend`] — re-exported from `progit-forge-client`. Talks
//!   to a sidecar daemon over gRPC. Useful for users who want sovereign
//!   centralised hosting (the marketing thesis).
//!
//! Both implement the same [`GitBackend`] trait. The TUI holds a
//! `Box<dyn GitBackend>` and stays oblivious to which one is wired.
//!
//! ## Why this module is feature-gated
//!
//! Pulling `gix` + `progit-forge-client` (and its `tonic` chain) into the
//! default TUI build adds ~3 MB. The doctrine target is a binary under 8 MB
//! stripped; default users who do not need this layer should not pay for it.
//! Activate with `cargo build --features forge-backend` or by setting the
//! feature in a downstream Cargo.toml.

use std::path::{Path, PathBuf};

use async_trait::async_trait;

// Re-export the trait + types so callers say `progit::git::backend::GitBackend`
// rather than reaching into progit-forge-client.
pub use progit_forge_client::backend::{
    BackendError, BackendResult, EphemeralBranch, GitBackend, PushOutcome,
    RefEntry, RefUpdate,
};
pub use progit_forge_client::ForgedBackend;

/// `GitBackend` impl that operates on a local-disk git directory.
///
/// Layout expected:
/// ```text
/// {root}/
///   {repo_name}/        bare git repo (created by `create_repo`)
///     HEAD
///     objects/
///       pack/           pack files persisted by `push`
///     refs/
///       heads/
/// ```
///
/// Each repo is a self-contained bare git directory. `create_repo` initialises
/// a fresh one; `delete_repo` removes the entire directory tree.
///
/// The implementation is `gix`-based. The TUI binary pays gix's cost only
/// when the `forge-backend` feature is enabled.
#[derive(Clone)]
pub struct LocalGitBackend {
    root: PathBuf,
}

impl LocalGitBackend {
    /// Open or create a backend rooted at `root`. The root directory is
    /// created if missing; per-repo subdirectories appear on `create_repo`.
    pub fn new(root: impl Into<PathBuf>) -> BackendResult<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)
            .map_err(|e| BackendError::Io(format!("create root {}: {e}", root.display())))?;
        Ok(Self { root })
    }

    fn repo_dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn pack_dir(&self, name: &str) -> PathBuf {
        self.repo_dir(name).join("objects").join("pack")
    }

    fn validate_repo_name(name: &str) -> BackendResult<()> {
        // Mirrors progit-forged's storage::validate_repo_name. Fail closed
        // on traversal sequences and bad characters.
        if name.is_empty() {
            return Err(BackendError::InvalidInput("repo name is empty".into()));
        }
        if name.len() > 200 {
            return Err(BackendError::InvalidInput("repo name exceeds 200 bytes".into()));
        }
        if name.starts_with('.') || name.starts_with('/') || name.starts_with('-') {
            return Err(BackendError::InvalidInput(format!(
                "repo name starts with reserved char: {name}"
            )));
        }
        if name.contains("..") || name.contains('\0') {
            return Err(BackendError::InvalidInput(format!(
                "repo name contains forbidden sequence: {name}"
            )));
        }
        for ch in name.chars() {
            let ok = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '.');
            if !ok {
                return Err(BackendError::InvalidInput(format!(
                    "repo name contains forbidden char: {ch:?}"
                )));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl GitBackend for LocalGitBackend {
    async fn create_repo(&self, name: &str) -> BackendResult<()> {
        Self::validate_repo_name(name)?;
        let dir = self.repo_dir(name);
        if dir.exists() {
            return Err(BackendError::RepoExists(name.into()));
        }
        std::fs::create_dir_all(&dir)
            .map_err(|e| BackendError::Io(format!("mkdir {}: {e}", dir.display())))?;

        // Initialise a bare git repo at this path.
        gix::init_bare(&dir)
            .map_err(|e| BackendError::Internal(format!("gix init_bare {}: {e}", dir.display())))?;

        Ok(())
    }

    async fn delete_repo(&self, name: &str) -> BackendResult<()> {
        Self::validate_repo_name(name)?;
        let dir = self.repo_dir(name);
        if !dir.exists() {
            return Err(BackendError::RepoNotFound(name.into()));
        }
        std::fs::remove_dir_all(&dir)
            .map_err(|e| BackendError::Io(format!("rm -rf {}: {e}", dir.display())))?;
        Ok(())
    }

    async fn list_refs(
        &self,
        repo: &str,
        prefix: Option<&str>,
    ) -> BackendResult<Vec<RefEntry>> {
        Self::validate_repo_name(repo)?;
        let dir = self.repo_dir(repo);
        if !dir.exists() {
            return Err(BackendError::RepoNotFound(repo.into()));
        }

        let r = gix::open(&dir).map_err(|e| {
            BackendError::Internal(format!("gix open {}: {e}", dir.display()))
        })?;
        let refs = r
            .references()
            .map_err(|e| BackendError::Internal(format!("gix references: {e}")))?;
        let all = refs
            .all()
            .map_err(|e| BackendError::Internal(format!("gix references all: {e}")))?;

        let mut out = Vec::new();
        for r in all {
            let r = r.map_err(|e| {
                BackendError::Internal(format!("gix reference iter: {e}"))
            })?;
            let name = r.name().as_bstr().to_string();
            if let Some(p) = prefix {
                if !name.starts_with(p) {
                    continue;
                }
            }
            // Resolve the OID. Symbolic refs (HEAD, etc.) are resolved to
            // their direct targets; if they cannot be resolved, skip.
            let oid_string = match r.target() {
                gix::refs::TargetRef::Object(id) => id.to_hex().to_string(),
                gix::refs::TargetRef::Symbolic(_) => continue,
            };
            out.push(RefEntry {
                name,
                oid: oid_string,
                ephemeral: false, // local backend has no TTL concept
            });
        }
        Ok(out)
    }

    async fn push(
        &self,
        repo: &str,
        updates: Vec<RefUpdate>,
        pack: Option<Vec<u8>>,
    ) -> BackendResult<PushOutcome> {
        Self::validate_repo_name(repo)?;
        let dir = self.repo_dir(repo);
        if !dir.exists() {
            return Err(BackendError::RepoNotFound(repo.into()));
        }
        let pack_dir = self.pack_dir(repo);
        std::fs::create_dir_all(&pack_dir).map_err(|e| {
            BackendError::Io(format!("mkdir {}: {e}", pack_dir.display()))
        })?;

        // ---- 1. Ingest the pack via gix-pack Bundle::write_to_directory.
        // Same pipeline as the daemon — header + entries + index.
        if let Some(bytes) = pack {
            ingest_pack(&pack_dir, &bytes)?;
        }

        // ---- 2. Open the repo for ref operations.
        let repository = gix::open(&dir).map_err(|e| {
            BackendError::Internal(format!("gix open {}: {e}", dir.display()))
        })?;

        let mut accepted = Vec::with_capacity(updates.len());
        let mut rejected = Vec::new();

        for upd in updates {
            let result = apply_ref_update(&repository, &upd);
            match result {
                Ok(()) => accepted.push(upd),
                Err(_) => rejected.push(upd),
            }
        }

        let ok = rejected.is_empty();
        let message = if ok {
            "ok".to_string()
        } else {
            format!(
                "{} of {} ref updates rejected",
                rejected.len(),
                accepted.len() + rejected.len()
            )
        };

        Ok(PushOutcome {
            ok,
            message,
            accepted,
            rejected,
        })
    }

    async fn fetch(
        &self,
        repo: &str,
        wants: Vec<String>,
    ) -> BackendResult<Vec<u8>> {
        Self::validate_repo_name(repo)?;
        let dir = self.repo_dir(repo);
        if !dir.exists() {
            return Err(BackendError::RepoNotFound(repo.into()));
        }
        let pack_dir = self.pack_dir(repo);

        // Validate every want is reachable in some pack. For local backends
        // this is a per-want lookup against each idx file in pack_dir.
        for want in &wants {
            if !oid_in_any_idx(&pack_dir, want)? {
                return Err(BackendError::InvalidInput(format!(
                    "wanted OID not in repo: {want}"
                )));
            }
        }

        // Mirror the daemon's v0.1.3.0 single-pack pass-through: if there
        // is exactly one pack, return its bytes; if zero, return a minimal
        // empty pack; otherwise Unsupported (matching the daemon's contract
        // saves consumers from having to special-case backends).
        let packs = list_packs(&pack_dir)?;
        match packs.len() {
            0 => Ok(empty_pack_bytes()),
            1 => std::fs::read(&packs[0]).map_err(|e| {
                BackendError::Io(format!("read pack {}: {e}", packs[0].display()))
            }),
            n => Err(BackendError::Unsupported(format!(
                "fetch from local repo with {n} packs — multi-pack repacking lands in v0.1.3.1"
            ))),
        }
    }

    // Default `create_ephemeral_branch` returns Unsupported. The local
    // backend has no TTL concept; ephemeral branches are a daemon feature.
}

// =====================================================================
// Helpers
// =====================================================================

/// Ingest a pack via `gix-pack::Bundle::write_to_directory`. Same pipeline
/// as `progit-forged::pack::PackIngestor::finalize`. Writes both the
/// `pack-{hash}.pack` and `pack-{hash}.idx` files.
fn ingest_pack(pack_dir: &Path, bytes: &[u8]) -> BackendResult<()> {
    use std::io::BufReader;
    use std::sync::atomic::AtomicBool;

    let interrupt = AtomicBool::new(false);
    let mut progress = gix_features::progress::Discard;
    let mut reader = BufReader::new(bytes);

    let outcome = gix_pack::Bundle::write_to_directory(
        &mut reader,
        Some(pack_dir),
        &mut progress,
        &interrupt,
        None::<&gix_object::find::Never>,
        gix_pack::bundle::write::Options::default(),
    )
    .map_err(|e| BackendError::InvalidInput(format!("pack rejected: {e}")))?;

    // Drop the `.keep` file the bundle writer leaves behind — we don't gate
    // refs through .keep here.
    if let Some(keep) = &outcome.keep_path {
        let _ = std::fs::remove_file(keep);
    }
    Ok(())
}

/// Apply one ref update to a gix repository. CAS via `expected_old_oid`.
///
/// CAS semantics intentionally mirror the daemon's `progit-forged`
/// storage layer (sled `compare_and_swap`):
///
/// - `old_oid = ""` (create-only): ref MUST NOT already exist. If it
///   does, regardless of what the new oid is, the update is rejected.
/// - `old_oid = "<hex>"` (update or delete): the ref MUST currently
///   point at exactly that oid. Mismatch or missing-ref → rejected.
///
/// gix's `edit_reference` with `PreviousValue::MustNotExist` is more
/// permissive than this — it accepts idempotent reapplies when the new
/// value matches an existing one. We pre-check explicitly to enforce
/// strict CAS uniformly across backends, so the same RefUpdate yields
/// the same outcome whether it routes through the daemon or local disk.
///
/// Returns `Err` on:
/// - CAS pre-image mismatch (`BackendError::CasFailed`)
/// - target OID not reachable in the repo's object DB (`BackendError::InvalidInput`)
/// - malformed inputs (`BackendError::InvalidInput`)
fn apply_ref_update(
    repo: &gix::Repository,
    upd: &RefUpdate,
) -> BackendResult<()> {
    use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};

    // ---- Parse + validate the ref name once.
    let ref_name: gix::refs::FullName = upd
        .ref_name
        .as_str()
        .try_into()
        .map_err(|e| BackendError::InvalidInput(format!("ref_name: {e}")))?;

    // ---- CAS pre-check. Look up the current ref state and enforce the
    // strict semantics described above before we touch the transaction.
    let existing_oid: Option<String> = match repo.try_find_reference(upd.ref_name.as_str()) {
        Ok(Some(r)) => match r.target() {
            gix::refs::TargetRef::Object(id) => Some(id.to_hex().to_string()),
            gix::refs::TargetRef::Symbolic(_) => {
                return Err(BackendError::CasFailed(format!(
                    "ref {} is symbolic; cannot CAS",
                    upd.ref_name
                )));
            }
        },
        Ok(None) => None,
        Err(e) => {
            return Err(BackendError::Internal(format!(
                "find_reference {}: {e}",
                upd.ref_name
            )));
        }
    };

    if upd.old_oid.is_empty() {
        // Caller said "create" — strict: must not currently exist.
        if existing_oid.is_some() {
            return Err(BackendError::CasFailed(format!(
                "ref {} already exists",
                upd.ref_name
            )));
        }
    } else {
        // Caller said "update" or "delete" — strict: ref must exist with
        // exactly the expected pre-image OID.
        let actual = existing_oid.ok_or_else(|| {
            BackendError::CasFailed(format!(
                "ref {} does not exist; CAS expects it to point at {}",
                upd.ref_name, upd.old_oid
            ))
        })?;
        if actual != upd.old_oid {
            return Err(BackendError::CasFailed(format!(
                "ref {} CAS mismatch: expected {}, got {}",
                upd.ref_name, upd.old_oid, actual
            )));
        }
    }

    // ---- Resolve target oid (or treat as deletion if empty).
    let new_oid = if upd.new_oid.is_empty() {
        None
    } else {
        let id = gix_hash::ObjectId::from_hex(upd.new_oid.as_bytes())
            .map_err(|e| BackendError::InvalidInput(format!("new_oid: {e}")))?;
        // OID-existence check: the target must be reachable. gix's
        // `try_find_header` looks across packs and loose objects.
        if repo.try_find_header(id).map_err(|e| {
            BackendError::Internal(format!("oid lookup: {e}"))
        })?.is_none() {
            return Err(BackendError::InvalidInput(format!(
                "ref target oid not in repo: {}",
                upd.new_oid
            )));
        }
        Some(id)
    };

    // ---- Build the previous-value expectation. We've already done the
    // strict check above; pass the same expectation through to gix as
    // a defense-in-depth layer (it'll catch the rare race window between
    // our pre-check and the transaction commit).
    let previous = if upd.old_oid.is_empty() {
        PreviousValue::MustNotExist
    } else {
        let old_id = gix_hash::ObjectId::from_hex(upd.old_oid.as_bytes())
            .map_err(|e| BackendError::InvalidInput(format!("old_oid: {e}")))?;
        PreviousValue::MustExistAndMatch(gix::refs::Target::Object(old_id))
    };

    // ---- Construct the edit.
    let change = match new_oid {
        Some(id) => Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: false,
                message: "progit local-backend update".into(),
            },
            expected: previous,
            new: gix::refs::Target::Object(id),
        },
        None => Change::Delete {
            expected: previous,
            log: RefLog::AndReference,
        },
    };
    let edit = RefEdit {
        change,
        name: ref_name,
        deref: false,
    };

    repo.edit_reference(edit).map_err(|e| {
        // gix surfaces CAS mismatches as RefEdit errors; map them all to
        // CasFailed so callers can retry with a refreshed pre-image.
        BackendError::CasFailed(format!("ref edit: {e}"))
    })?;
    Ok(())
}

/// List canonical `pack-*.pack` files in a directory (no `.idx`, no
/// `incoming-*` temps, no stray files), sorted by name.
fn list_packs(pack_dir: &Path) -> BackendResult<Vec<PathBuf>> {
    if !pack_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(pack_dir)
        .map_err(|e| BackendError::Io(format!("readdir {}: {e}", pack_dir.display())))?
    {
        let entry = entry.map_err(|e| BackendError::Io(format!("readdir entry: {e}")))?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if name.starts_with("pack-") && name.ends_with(".pack") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Look up an OID hex string in any `*.idx` under `pack_dir`. Cheap —
/// each idx is opened and binary-searched.
fn oid_in_any_idx(pack_dir: &Path, oid_hex: &str) -> BackendResult<bool> {
    if !pack_dir.exists() {
        return Ok(false);
    }
    let object_id = match gix_hash::ObjectId::from_hex(oid_hex.as_bytes()) {
        Ok(id) => id,
        Err(_) => return Ok(false),
    };
    for entry in std::fs::read_dir(pack_dir)
        .map_err(|e| BackendError::Io(format!("readdir: {e}")))?
    {
        let entry = entry.map_err(|e| BackendError::Io(format!("readdir entry: {e}")))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("idx") {
            continue;
        }
        let idx = match gix_pack::index::File::at(&path, gix_hash::Kind::Sha1) {
            Ok(f) => f,
            Err(_) => continue,
        };
        if idx.lookup(&object_id).is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Build a minimal valid empty pack — same shape the daemon emits for
/// 0-pack repos: 12-byte header + 20-byte SHA-1 trailer = 32 bytes.
fn empty_pack_bytes() -> Vec<u8> {
    use sha1::{Digest, Sha1};
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(b"PACK");
    bytes.extend_from_slice(&2u32.to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());

    let mut h = Sha1::new();
    h.update(&bytes);
    let trailer: [u8; 20] = h.finalize().into();
    bytes.extend_from_slice(&trailer);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> (tempfile::TempDir, LocalGitBackend) {
        let tmp = tempfile::TempDir::new().unwrap();
        let backend = LocalGitBackend::new(tmp.path()).unwrap();
        (tmp, backend)
    }

    #[tokio::test]
    async fn create_repo_makes_bare_git_layout() {
        let (tmp, backend) = fresh();
        backend.create_repo("hello").await.unwrap();
        let repo_dir = tmp.path().join("hello");
        assert!(repo_dir.exists());
        assert!(repo_dir.join("HEAD").exists());
        assert!(repo_dir.join("objects").exists());
        assert!(repo_dir.join("refs").exists());
    }

    #[tokio::test]
    async fn create_repo_already_exists() {
        let (_tmp, backend) = fresh();
        backend.create_repo("dup").await.unwrap();
        let err = backend.create_repo("dup").await.unwrap_err();
        assert!(matches!(err, BackendError::RepoExists(_)));
    }

    #[tokio::test]
    async fn list_refs_unknown_repo_is_not_found() {
        let (_tmp, backend) = fresh();
        let err = backend.list_refs("ghost", None).await.unwrap_err();
        assert!(matches!(err, BackendError::RepoNotFound(_)));
    }

    #[tokio::test]
    async fn list_refs_empty_repo_is_empty() {
        let (_tmp, backend) = fresh();
        backend.create_repo("e").await.unwrap();
        let refs = backend.list_refs("e", None).await.unwrap();
        assert!(refs.is_empty());
    }

    #[tokio::test]
    async fn delete_repo_round_trip() {
        let (_tmp, backend) = fresh();
        backend.create_repo("d").await.unwrap();
        backend.delete_repo("d").await.unwrap();
        let err = backend.list_refs("d", None).await.unwrap_err();
        assert!(matches!(err, BackendError::RepoNotFound(_)));
    }

    #[tokio::test]
    async fn validates_repo_name() {
        let (_tmp, backend) = fresh();
        for bad in ["..", "../escape", "a/../b", "with\0null", "", ".hidden", "-leading"] {
            let err = backend.create_repo(bad).await.unwrap_err();
            assert!(matches!(err, BackendError::InvalidInput(_)),
                "expected InvalidInput for {bad:?}, got {err:?}");
        }
    }

    #[tokio::test]
    async fn ephemeral_branch_unsupported_by_default_impl() {
        let (_tmp, backend) = fresh();
        backend.create_repo("e").await.unwrap();
        let err = backend
            .create_ephemeral_branch("e", &"a".repeat(40), 60, None)
            .await
            .unwrap_err();
        assert!(matches!(err, BackendError::Unsupported(_)));
    }

    #[tokio::test]
    async fn fetch_empty_repo_returns_minimal_pack() {
        let (_tmp, backend) = fresh();
        backend.create_repo("e").await.unwrap();
        let bytes = backend.fetch("e", vec![]).await.unwrap();
        assert_eq!(bytes.len(), 32);
        assert!(bytes.starts_with(b"PACK"));
    }

    #[tokio::test]
    async fn fetch_unknown_oid_is_invalid_input() {
        let (_tmp, backend) = fresh();
        backend.create_repo("u").await.unwrap();
        let err = backend
            .fetch("u", vec!["a".repeat(40)])
            .await
            .unwrap_err();
        assert!(matches!(err, BackendError::InvalidInput(_)));
    }

    /// End-to-end: push a real pack containing one blob, list_refs sees
    /// the new ref, fetch returns pack bytes. Reuses the daemon's pack
    /// fixture builder (which is `pub` for exactly this kind of cross-crate
    /// testing).
    ///
    /// This test depends on `progit-forged` for the fixture only — we
    /// could carve it out into its own helper crate later if dev-dep
    /// becomes inconvenient. For now: simple.
    #[tokio::test]
    async fn push_pack_then_list_refs_then_fetch() {
        // Skip this integration if progit-forged isn't available as a dev
        // dep. Rather than weave in a build switch, build the fixture
        // inline using the same algorithm.
        let (pack, oid) = build_pack_with_blob(b"local-backend round-trip");

        let (_tmp, backend) = fresh();
        backend.create_repo("rt").await.unwrap();

        let outcome = backend
            .push(
                "rt",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: String::new(),
                    new_oid: oid.clone(),
                }],
                Some(pack),
            )
            .await
            .unwrap();
        assert!(outcome.ok, "push rejected: {}", outcome.message);
        assert_eq!(outcome.accepted.len(), 1);

        // list_refs sees the new ref.
        let refs = backend
            .list_refs("rt", Some("refs/heads/"))
            .await
            .unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "refs/heads/main");
        assert_eq!(refs[0].oid, oid);

        // fetch returns pack bytes.
        let bytes = backend.fetch("rt", vec![]).await.unwrap();
        assert!(bytes.starts_with(b"PACK"));
        assert!(bytes.len() >= 32);
    }

    #[tokio::test]
    async fn second_create_on_existing_ref_is_cas_failed() {
        // Strict CAS: second push of the same ref with empty old_oid
        // (create-only) must reject because the ref already exists. This
        // matches the daemon's sled-backed update_ref semantics.
        let (_tmp, backend) = fresh();
        backend.create_repo("c").await.unwrap();
        let (pack, oid) = build_pack_with_blob(b"twice over");

        // First push lands the ref.
        let first = backend
            .push(
                "c",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: String::new(),
                    new_oid: oid.clone(),
                }],
                Some(pack),
            )
            .await
            .unwrap();
        assert!(first.ok);
        assert_eq!(first.accepted.len(), 1);

        // Second push with the same OID and old_oid="" must be rejected.
        let second = backend
            .push(
                "c",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: String::new(),
                    new_oid: oid.clone(),
                }],
                None,
            )
            .await
            .unwrap();
        assert!(!second.ok, "second create-only push must reject");
        assert_eq!(second.accepted.len(), 0);
        assert_eq!(second.rejected.len(), 1);
    }

    #[tokio::test]
    async fn update_with_correct_old_oid_succeeds() {
        // Positive CAS: with the correct pre-image OID, an update goes through.
        let (_tmp, backend) = fresh();
        backend.create_repo("u").await.unwrap();

        let (pack_a, oid_a) = build_pack_with_blob(b"first");
        backend
            .push(
                "u",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: String::new(),
                    new_oid: oid_a.clone(),
                }],
                Some(pack_a),
            )
            .await
            .unwrap();

        let (pack_b, oid_b) = build_pack_with_blob(b"second");
        let outcome = backend
            .push(
                "u",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: oid_a, // correct pre-image
                    new_oid: oid_b.clone(),
                }],
                Some(pack_b),
            )
            .await
            .unwrap();
        assert!(outcome.ok);
        assert_eq!(outcome.accepted.len(), 1);

        // Ref now points at oid_b.
        let refs = backend.list_refs("u", Some("refs/heads/")).await.unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].oid, oid_b);
    }

    #[tokio::test]
    async fn update_with_wrong_old_oid_is_cas_failed() {
        // Negative CAS: pre-image OID doesn't match → reject.
        let (_tmp, backend) = fresh();
        backend.create_repo("w").await.unwrap();

        let (pack, oid) = build_pack_with_blob(b"present");
        backend
            .push(
                "w",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: String::new(),
                    new_oid: oid.clone(),
                }],
                Some(pack),
            )
            .await
            .unwrap();

        // Try to update with the wrong pre-image.
        let outcome = backend
            .push(
                "w",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: "0".repeat(40), // wrong pre-image
                    new_oid: oid.clone(),
                }],
                None,
            )
            .await
            .unwrap();
        assert!(!outcome.ok);
        assert_eq!(outcome.rejected.len(), 1);

        // Ref still points at the original OID.
        let refs = backend.list_refs("w", Some("refs/heads/")).await.unwrap();
        assert_eq!(refs[0].oid, oid);
    }

    #[tokio::test]
    async fn update_on_missing_ref_is_cas_failed() {
        // Negative CAS: caller specifies an old_oid but ref doesn't exist.
        let (_tmp, backend) = fresh();
        backend.create_repo("m").await.unwrap();
        let (pack, oid) = build_pack_with_blob(b"unattached");
        backend
            .push(
                "m",
                vec![RefUpdate {
                    ref_name: "refs/heads/scratch".into(),
                    old_oid: String::new(),
                    new_oid: oid.clone(),
                }],
                Some(pack),
            )
            .await
            .unwrap();

        // Try to update refs/heads/main (doesn't exist) with a pre-image.
        let outcome = backend
            .push(
                "m",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: oid.clone(),
                    new_oid: oid,
                }],
                None,
            )
            .await
            .unwrap();
        assert!(!outcome.ok);
        assert_eq!(outcome.rejected.len(), 1);
    }

    #[tokio::test]
    async fn header_only_push_with_unknown_oid_is_rejected() {
        let (_tmp, backend) = fresh();
        backend.create_repo("u").await.unwrap();
        let outcome = backend
            .push(
                "u",
                vec![RefUpdate {
                    ref_name: "refs/heads/main".into(),
                    old_oid: String::new(),
                    new_oid: "f".repeat(40),
                }],
                None,
            )
            .await
            .unwrap();
        assert!(!outcome.ok);
        assert_eq!(outcome.accepted.len(), 0);
        assert_eq!(outcome.rejected.len(), 1);
    }

    // ----- helpers (pack fixture, inlined to avoid a hard dev-dep on
    //       progit-forged from this module's tests) -----

    fn build_pack_with_blob(content: &[u8]) -> (Vec<u8>, String) {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use sha1::{Digest, Sha1};
        use std::io::Write;

        // Compute blob OID — git's standard "blob {len}\0{content}" form.
        let mut h = Sha1::new();
        h.update(format!("blob {}", content.len()).as_bytes());
        h.update(b"\0");
        h.update(content);
        let oid: [u8; 20] = h.finalize().into();
        let oid_hex = hex::encode(oid);

        // Entry header: variable-length size encoding, type=Blob (3).
        let blob_type: u8 = 3;
        let size = content.len() as u64;
        let mut hdr = Vec::new();
        let mut first = (blob_type << 4) | ((size & 0x0F) as u8);
        let mut s = size >> 4;
        while s > 0 {
            first |= 0x80;
            hdr.push(first);
            first = (s & 0x7F) as u8;
            s >>= 7;
        }
        hdr.push(first);

        // zlib-compress the blob body.
        let mut zlib = ZlibEncoder::new(Vec::new(), Compression::default());
        zlib.write_all(content).unwrap();
        let deflated = zlib.finish().unwrap();

        // Pack: header + entry + trailer.
        let mut pack = Vec::new();
        pack.extend_from_slice(b"PACK");
        pack.extend_from_slice(&2u32.to_be_bytes());
        pack.extend_from_slice(&1u32.to_be_bytes());
        pack.extend_from_slice(&hdr);
        pack.extend_from_slice(&deflated);

        let mut h = Sha1::new();
        h.update(&pack);
        let trailer: [u8; 20] = h.finalize().into();
        pack.extend_from_slice(&trailer);

        (pack, oid_hex)
    }
}
