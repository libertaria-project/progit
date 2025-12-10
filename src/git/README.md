# Git Feature

Git repository integration for ProjectsTUI.

## Structure

| File | Purpose |
|------|---------|
| `repository.rs` | Repository detection, remote info |
| `widget_gitbar.rs` | Top bar showing repo status |

## Features

- Auto-detect git repository from current directory
- Show branch name, remote URL
- Switch between remotes (if multiple)
- Display sync status (ahead/behind)

## Public API

```rust
use projectstui::git::{Repository, detect_repo};
```
