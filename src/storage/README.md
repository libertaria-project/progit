# Storage Feature

Dual-format persistence: KDL for humans (carbons), JSON for machines (silicons).

## Structure

| File | Purpose |
|------|---------|
| `kdl.rs` | Read/write KDL files (human-editable, git-tracked) |
| `json.rs` | Read/write JSON cache (machine-fast, gitignored) |
| `sync.rs` | Sync KDL → JSON on file changes |
| `test_storage.rs` | Colocated tests |

## Philosophy

- **KDL is source of truth** - git-diffable, human-readable
- **JSON is cache** - fast loading, regenerated on startup

## File Locations

```
.projects/
├── config.kdl          # Sprint settings
├── issues/             # KDL files (one per issue)
│   └── *.kdl
└── .cache/             # Gitignored
    └── issues.json     # JSON cache
```
