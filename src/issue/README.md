# Issue Feature

The atomic unit of work in ProjectsTUI.

## Structure

| File | Purpose |
|------|---------|
| `model.rs` | `Issue` struct, `Status` enum, `Effort` type |
| `operations.rs` | CRUD: create, update, delete, load_all |
| `query.rs` | Filters, search, sort operations |
| `test_operations.rs` | Colocated unit tests |

## Public API (via `issue.rs`)

```rust
use projectstui::issue::{Issue, Status, create, update, delete};
```

## Data Model

```kdl
issue id="abc-123" {
    title "Fix authentication leak"
    status "in-progress"
    effort 5
    tags { - "backend" - "blocker" }
}
```
