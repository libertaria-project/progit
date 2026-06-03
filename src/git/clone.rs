// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2026 Markus Maiwald

//! Clone-via-trait — first call site exercising the `GitBackend` abstraction.
//!
//! [`clone_repo`] is generic over `&dyn GitBackend` for both source and
//! destination. The same code path drives:
//!
//! - daemon → local (the canonical first-clone use case)
//! - local → daemon (uploading a local repo to a sovereign forge)
//! - daemon → daemon (federation between forges)
//!
//! This is the load-bearing demonstration of the trait abstraction. If
//! `clone_repo` works against any pair of backends, the trait is honest —
//! it isn't accidentally daemon-shaped or local-shaped.

use anyhow::{Context, Result};

use crate::git::backend::{BackendError, GitBackend, RefUpdate};

/// Outcome of a clone operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneOutcome {
    /// Number of refs found on the source.
    pub refs_total: usize,
    /// Number of refs the destination accepted.
    pub refs_accepted: usize,
    /// Number of refs the destination rejected (CAS / OID-existence / validation).
    pub refs_rejected: usize,
    /// Pack bytes streamed from source to destination.
    pub pack_bytes: usize,
}

/// Clone every ref + pack from `source[source_repo]` into `dest[dest_repo]`.
///
/// The destination repo is created if it does not already exist; an existing
/// repo is reused (existing refs may be left in place — no destructive sync).
///
/// # Notes
///
/// - For v0.1 this is a **deep clone**: empty `want_oids` means "give me
///   everything." Once daemons support `have_oids` filtering, an incremental
///   variant of this function will join.
/// - Refs are pushed with `old_oid = ""` (create-only). If the destination
///   already has a ref of the same name pointing at a different oid, that
///   ref will be rejected with a CAS mismatch — surfaced in
///   `CloneOutcome.refs_rejected`. Callers can decide whether that is fatal.
pub async fn clone_repo(
    source: &dyn GitBackend,
    source_repo: &str,
    dest: &dyn GitBackend,
    dest_repo: &str,
) -> Result<CloneOutcome> {
    // 1. Read refs from source.
    let source_refs = source
        .list_refs(source_repo, None)
        .await
        .with_context(|| format!("listing refs on source repo '{source_repo}'"))?;

    // 2. Fetch pack bytes (deep clone).
    let pack_bytes = source
        .fetch(source_repo, vec![])
        .await
        .with_context(|| format!("fetching pack from source repo '{source_repo}'"))?;

    // 3. Initialise destination repo. Idempotent on RepoExists.
    match dest.create_repo(dest_repo).await {
        Ok(()) => {}
        Err(BackendError::RepoExists(_)) => {
            // Reuse existing destination — caller is responsible for any
            // pre-existing conflicting refs.
        }
        Err(e) => {
            return Err(anyhow::anyhow!("create_repo on dest '{dest_repo}': {e}"));
        }
    }

    // 4. Push pack + refs into destination via the trait.
    let updates: Vec<RefUpdate> = source_refs
        .iter()
        .map(|r| RefUpdate {
            ref_name: r.name.clone(),
            old_oid: String::new(), // create-only
            new_oid: r.oid.clone(),
        })
        .collect();

    let pack_for_push = if pack_bytes.is_empty() || pack_bytes.len() == 32 {
        // 32-byte minimal empty pack: skip — the destination would accept it
        // but there's nothing to ingest. Cleaner to send None.
        None
    } else {
        Some(pack_bytes.clone())
    };

    let outcome = dest
        .push(dest_repo, updates, pack_for_push)
        .await
        .with_context(|| format!("pushing into dest '{dest_repo}'"))?;

    Ok(CloneOutcome {
        refs_total: source_refs.len(),
        refs_accepted: outcome.accepted.len(),
        refs_rejected: outcome.rejected.len(),
        pack_bytes: pack_bytes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::backend::{LocalGitBackend, RefEntry};

    /// Trivial in-memory backend that lets us drive `clone_repo` purely in
    /// process. Models a daemon-like surface without spinning up a daemon.
    /// Stores: per-repo (Vec<RefEntry>, Option<Vec<u8>> pack).
    struct InMemoryBackend {
        repos:
            tokio::sync::Mutex<std::collections::HashMap<String, (Vec<RefEntry>, Option<Vec<u8>>)>>,
    }

    impl InMemoryBackend {
        fn new() -> Self {
            Self {
                repos: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }

        async fn seed(&self, repo: &str, refs: Vec<RefEntry>, pack: Option<Vec<u8>>) {
            self.repos.lock().await.insert(repo.into(), (refs, pack));
        }
    }

    #[async_trait::async_trait]
    impl GitBackend for InMemoryBackend {
        async fn create_repo(&self, name: &str) -> crate::git::backend::BackendResult<()> {
            let mut repos = self.repos.lock().await;
            if repos.contains_key(name) {
                return Err(BackendError::RepoExists(name.into()));
            }
            repos.insert(name.into(), (Vec::new(), None));
            Ok(())
        }
        async fn delete_repo(&self, name: &str) -> crate::git::backend::BackendResult<()> {
            let mut repos = self.repos.lock().await;
            repos
                .remove(name)
                .map(|_| ())
                .ok_or_else(|| BackendError::RepoNotFound(name.into()))
        }
        async fn list_refs(
            &self,
            repo: &str,
            prefix: Option<&str>,
        ) -> crate::git::backend::BackendResult<Vec<RefEntry>> {
            let repos = self.repos.lock().await;
            let (refs, _) = repos
                .get(repo)
                .ok_or_else(|| BackendError::RepoNotFound(repo.into()))?;
            Ok(refs
                .iter()
                .filter(|r| match prefix {
                    Some(p) => r.name.starts_with(p),
                    None => true,
                })
                .cloned()
                .collect())
        }
        async fn push(
            &self,
            repo: &str,
            updates: Vec<RefUpdate>,
            pack: Option<Vec<u8>>,
        ) -> crate::git::backend::BackendResult<crate::git::backend::PushOutcome> {
            let mut repos = self.repos.lock().await;
            let (refs, existing_pack) = repos
                .get_mut(repo)
                .ok_or_else(|| BackendError::RepoNotFound(repo.into()))?;
            if let Some(p) = pack {
                *existing_pack = Some(p);
            }
            let mut accepted = Vec::new();
            for upd in updates {
                refs.push(RefEntry {
                    name: upd.ref_name.clone(),
                    oid: upd.new_oid.clone(),
                    ephemeral: false,
                });
                accepted.push(upd);
            }
            Ok(crate::git::backend::PushOutcome {
                ok: true,
                message: "ok".into(),
                accepted,
                rejected: vec![],
            })
        }
        async fn fetch(
            &self,
            repo: &str,
            _wants: Vec<String>,
        ) -> crate::git::backend::BackendResult<Vec<u8>> {
            let repos = self.repos.lock().await;
            let (_, pack) = repos
                .get(repo)
                .ok_or_else(|| BackendError::RepoNotFound(repo.into()))?;
            Ok(pack.clone().unwrap_or_else(|| {
                // Empty pack
                let mut bytes = Vec::with_capacity(32);
                bytes.extend_from_slice(b"PACK");
                bytes.extend_from_slice(&2u32.to_be_bytes());
                bytes.extend_from_slice(&0u32.to_be_bytes());
                bytes.extend_from_slice(&[0u8; 20]);
                bytes
            }))
        }
    }

    #[tokio::test]
    async fn clone_in_memory_to_in_memory_via_trait() {
        // Source: in-memory backend with one ref + pack.
        let source = InMemoryBackend::new();
        source
            .seed(
                "src",
                vec![RefEntry {
                    name: "refs/heads/main".into(),
                    oid: "f".repeat(40),
                    ephemeral: false,
                }],
                Some(b"FAKE_PACK_BYTES".to_vec()),
            )
            .await;

        let dest = InMemoryBackend::new();

        let outcome = clone_repo(&source, "src", &dest, "dst").await.unwrap();
        assert_eq!(outcome.refs_total, 1);
        assert_eq!(outcome.refs_accepted, 1);
        assert_eq!(outcome.refs_rejected, 0);

        // Destination has the same ref under the new name.
        let refs = dest.list_refs("dst", None).await.unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "refs/heads/main");
        assert_eq!(refs[0].oid, "f".repeat(40));
    }

    #[tokio::test]
    async fn clone_into_existing_dest_is_idempotent() {
        // If the dest repo already exists, clone should reuse it.
        let source = InMemoryBackend::new();
        source.seed("src", vec![], None).await;
        let dest = InMemoryBackend::new();
        dest.create_repo("dst").await.unwrap();

        let outcome = clone_repo(&source, "src", &dest, "dst").await.unwrap();
        assert_eq!(outcome.refs_total, 0);
    }

    #[tokio::test]
    async fn clone_to_local_disk_backend_through_dyn_trait() {
        // The trait-object property: clone takes &dyn GitBackend, so any
        // mix of concrete impls composes. Source = in-memory, dest = local-disk.
        let source = InMemoryBackend::new();
        // Seed with no pack — clone will pass None pack to dest.
        source.seed("tiny", vec![], None).await;

        let tmp = tempfile::TempDir::new().unwrap();
        let dest = LocalGitBackend::new(tmp.path()).unwrap();

        let src_dyn: &dyn GitBackend = &source;
        let dst_dyn: &dyn GitBackend = &dest;

        let outcome = clone_repo(src_dyn, "tiny", dst_dyn, "tiny").await.unwrap();
        assert_eq!(outcome.refs_total, 0);

        // Local dest now exists.
        let refs = dest.list_refs("tiny", None).await.unwrap();
        assert!(refs.is_empty());
    }
}
