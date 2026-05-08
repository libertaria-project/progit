// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2026 Markus Maiwald

//! End-to-end test: clone from an in-process `progit-forged` daemon into a
//! `LocalGitBackend`, both behind the `GitBackend` trait. This is the
//! validation test for the trait abstraction — the same `clone_repo`
//! function works against any pair of backends.
//!
//! Gated on `forge-backend`; with the feature off, this file is a no-op.

#![cfg(feature = "forge-backend")]

use std::sync::Arc;

use progit::git::backend::{ForgedBackend, GitBackend, LocalGitBackend, RefUpdate};
use progit::git::clone::clone_repo;
use progit_forged::rpc::{ForgeServer, ForgeService};
use progit_forged::storage::Store;
use tokio::net::TcpListener;
use tonic::transport::Server;

/// Spin up a daemon on a random port. Returns its URL plus the temp dir.
async fn spawn_daemon() -> (String, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().unwrap();
    let store = Arc::new(Store::open(tmp.path()).unwrap());
    let svc = ForgeService::new(store);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stream = tokio_stream::wrappers::TcpListenerStream::new(listener);

    tokio::spawn(async move {
        Server::builder()
            .add_service(ForgeServer::new(svc))
            .serve_with_incoming(stream)
            .await
            .unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    (format!("http://{addr}"), tmp)
}

#[tokio::test]
async fn clone_from_daemon_to_local_backend() {
    // ---- 1. Set up daemon, push a real pack containing one blob.
    let (url, _daemon_tmp) = spawn_daemon().await;
    let daemon = ForgedBackend::connect(url.clone()).await.unwrap();
    daemon.create_repo("source-repo").await.unwrap();

    let (pack, blob_oid) =
        progit_forged::pack::build_pack_with_blob(b"clone-via-trait demo");
    let outcome = daemon
        .push(
            "source-repo",
            vec![RefUpdate {
                ref_name: "refs/heads/main".into(),
                old_oid: String::new(),
                new_oid: blob_oid.clone(),
            }],
            Some(pack.clone()),
        )
        .await
        .unwrap();
    assert!(outcome.ok, "seed push rejected: {}", outcome.message);

    // ---- 2. Clone the daemon repo into a fresh local backend.
    //         Both source and dest are &dyn GitBackend — same trait.
    let local_root = tempfile::TempDir::new().unwrap();
    let local = LocalGitBackend::new(local_root.path()).unwrap();

    let result = clone_repo(&daemon, "source-repo", &local, "cloned").await.unwrap();
    assert_eq!(result.refs_total, 1, "expected one ref on source");
    assert_eq!(result.refs_accepted, 1, "expected one accepted ref on dest");
    assert_eq!(result.refs_rejected, 0);
    assert!(result.pack_bytes >= 32, "expected pack bytes streamed");

    // ---- 3. Verify the local backend now contains the cloned data.
    let local_refs = local.list_refs("cloned", Some("refs/heads/")).await.unwrap();
    assert_eq!(local_refs.len(), 1, "local has exactly one head");
    assert_eq!(local_refs[0].name, "refs/heads/main");
    assert_eq!(
        local_refs[0].oid, blob_oid,
        "local ref points at the same OID as the source"
    );

    // ---- 4. Dest pack file is on disk under the local backend's layout.
    let pack_dir = local_root
        .path()
        .join("cloned")
        .join("objects")
        .join("pack");
    let pack_files: Vec<_> = std::fs::read_dir(&pack_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name();
            let s = n.to_string_lossy();
            s.starts_with("pack-") && s.ends_with(".pack")
        })
        .collect();
    assert_eq!(pack_files.len(), 1, "exactly one pack file on local disk");

    // ---- 5. Idempotency: a second clone into the same dest with the
    //         create-only (old_oid="") refs the trait emits hits strict
    //         CAS — the refs already exist on dest. Both backends now
    //         enforce this uniformly (the daemon via sled, LocalGitBackend
    //         via the explicit pre-check in apply_ref_update). The second
    //         clone's outcome reports the existing refs as rejected.
    let second = clone_repo(&daemon, "source-repo", &local, "cloned").await.unwrap();
    assert_eq!(second.refs_total, 1);
    assert_eq!(
        second.refs_rejected, 1,
        "second clone refs rejected — refs already exist; strict CAS uniformly enforced"
    );
    assert_eq!(second.refs_accepted, 0);
}

#[tokio::test]
async fn clone_from_local_to_local_via_trait() {
    // Local-to-local clone — proves the trait abstraction handles this
    // case too (even though there's no obvious user story for it; it's a
    // shape test).
    let src_root = tempfile::TempDir::new().unwrap();
    let src = LocalGitBackend::new(src_root.path()).unwrap();
    src.create_repo("a").await.unwrap();

    // Push a pack into the source local backend.
    let (pack, oid) = progit_forged::pack::build_pack_with_blob(b"local source");
    src.push(
        "a",
        vec![RefUpdate {
            ref_name: "refs/heads/main".into(),
            old_oid: String::new(),
            new_oid: oid.clone(),
        }],
        Some(pack),
    )
    .await
    .unwrap();

    // Clone to a second local backend.
    let dst_root = tempfile::TempDir::new().unwrap();
    let dst = LocalGitBackend::new(dst_root.path()).unwrap();

    let result = clone_repo(&src, "a", &dst, "a-clone").await.unwrap();
    assert_eq!(result.refs_accepted, 1);

    let refs = dst.list_refs("a-clone", None).await.unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].oid, oid);
}

#[tokio::test]
async fn clone_from_multi_pack_daemon_repo() {
    // Regression for the daemon's v0.1.3.1 multi-pack Fetch landing.
    // Before that, this test would have failed with Unsupported on the
    // second-pack fetch; now the trait abstraction propagates the daemon
    // improvement transparently to every consumer — the TUI didn't need
    // to change.
    let (url, _daemon_tmp) = spawn_daemon().await;
    let daemon = ForgedBackend::connect(url).await.unwrap();
    daemon.create_repo("multi").await.unwrap();

    // Two distinct pushes → two pack files in the source repo.
    let mut pushed_oids = Vec::new();
    for (i, blob) in [(0, b"first half" as &[u8]), (1, b"second half")].iter() {
        let (pack, oid) = progit_forged::pack::build_pack_with_blob(blob);
        pushed_oids.push(oid.clone());
        let outcome = daemon
            .push(
                "multi",
                vec![RefUpdate {
                    ref_name: format!("refs/heads/branch-{i}"),
                    old_oid: String::new(),
                    new_oid: oid,
                }],
                Some(pack),
            )
            .await
            .unwrap();
        assert!(outcome.ok, "seed push {i} rejected: {}", outcome.message);
    }

    // Clone — via the trait — into a local backend. This used to fail
    // with Unsupported; now it returns a combined pack that the local
    // backend ingests like any other.
    let local_root = tempfile::TempDir::new().unwrap();
    let local = LocalGitBackend::new(local_root.path()).unwrap();
    let result = clone_repo(&daemon, "multi", &local, "multi-cloned")
        .await
        .unwrap();
    assert_eq!(result.refs_total, 2, "two refs on source");
    assert_eq!(result.refs_accepted, 2, "both refs accepted on dest");
    assert!(result.pack_bytes >= 64, "non-trivial pack streamed");

    // Both refs landed on the destination, pointing at the original OIDs.
    let local_refs = local
        .list_refs("multi-cloned", Some("refs/heads/"))
        .await
        .unwrap();
    assert_eq!(local_refs.len(), 2);
    let mut local_oids: Vec<_> = local_refs.iter().map(|r| r.oid.clone()).collect();
    local_oids.sort();
    let mut expected = pushed_oids.clone();
    expected.sort();
    assert_eq!(local_oids, expected, "destination refs match source OIDs");
}

#[tokio::test]
async fn clone_unknown_source_repo_is_a_clean_error() {
    let (url, _daemon_tmp) = spawn_daemon().await;
    let daemon = ForgedBackend::connect(url).await.unwrap();

    let local_root = tempfile::TempDir::new().unwrap();
    let local = LocalGitBackend::new(local_root.path()).unwrap();

    let err = clone_repo(&daemon, "nonexistent", &local, "x")
        .await
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("listing refs") && msg.contains("not found"),
        "expected a wrapped 'list refs not found' error, got: {msg}"
    );
}
