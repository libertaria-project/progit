# Implementation Plan - Storage Engine Overhaul

## Problem Analysis
The current storage system relies on ad-hoc synchronization between KDL source files and a JSON cache. This has led to:
- **Ghost IDs**: Issues regenerating IDs upon read because the parser failed to find existing ones.
- **Deletion Failures**: The deletion logic couldn't reliably map IDs to files due to inconsistent parsing/generation.
- **Untestable Code**: Core logic is trapped in a binary crate (`src/main.rs`), making integration tests (`tests/*.rs`) impossible to run against internal modules.

## Proposed Architecture

### 1. Structural Refactor (Lib/Bin Split)
We will separate the core business logic from the CLI/TUI entry point.
- **`src/lib.rs`**: Will expose `issue`, `storage`, `sync`, `git` modules.
- **`src/main.rs`**: Will be a thin wrapper around `lib.rs`.
- **Impact**: Enables `tests/` to actually import `progit::*` and verify fixes.

### 2. Storage Engine Redesign (`src/storage/engine.rs`)
Instead of loose functions, we will implement a `StorageEngine` struct that manages the lifecycle of issues.

```rust
pub struct StorageEngine {
    root: PathBuf,
    issues_dir: PathBuf,
    cache_path: PathBuf,
}

impl StorageEngine {
    pub fn new(root: PathBuf) -> Self;
    pub fn load_all(&self) -> Result<Vec<Issue>>;
    pub fn save(&self, issue: &Issue) -> Result<()>;
    pub fn delete(&self, issue_id: &str) -> Result<()>;
    fn reindex(&self) -> Result<()>; // The source of truth for cache sync
}
```

### 3. KDL Parser Robustness
- strictly enforce `issue { id "..." }` or `issue id="..."` support.
- **Validation**: Fail loudly if an ID is missing during a *write* operation, but handle legacy reads gracefully (and fix them).
- **File Naming**: Enforce `[id]-[slug].kdl` naming to make filesystem lookups deterministic (avoid scanning all files for deletions).

### 4. Atomic Operations
- **Save**: Write to `.tmp`, rename to `.kdl` (atomic).
- **Delete**: Locate by ID (via Index or deterministic name), remove file, update Index.
- **Index**: The JSON cache should be treated purely as a read-optimization, not a database. It gets rebuilt if it's stale or corrupt.

## Execution Steps

1.  **Refactor Crate**: Create `src/lib.rs` and move modules. Update `Cargo.toml`.
2.  **Fix Parser**: Implement the robust `parse_kdl` (from our diagnosis) in the new lib.
3.  **Implement `StorageEngine`**: Centralize logic.
4.  **Update Consumers**: Refactor `main.rs`, `tui/app.rs`, and `cli` to use `StorageEngine`.
5.  **Verify**: Run the `repro_ghost_delete.rs` test (which will finally compile).

## Checklist
- [ ] Create `src/lib.rs`
- [ ] Update `Cargo.toml`
- [ ] Implement `StorageEngine`
- [ ] Refactor `parse_kdl`
- [ ] Update `main.rs` to use lib
- [ ] Pass `repro_ghost_delete.rs`
