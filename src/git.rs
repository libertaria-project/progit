//! Git Feature - Sovereign Index
//!
//! Git repository integration for issue tracking.
//! All git logic lives in `git/` folder.

pub mod blame;
pub mod repository;
pub mod widget_gitbar;

// Optional GitBackend trait + LocalGitBackend (gix) + ForgedBackend
// (re-exported from progit-forge-client). Enabled with --features=forge-backend.
#[cfg(feature = "forge-backend")]
pub mod backend;

// Re-export public API
pub use repository::{
    create_branch, create_remote_branch, delete_branch, detect_repo,
    get_origin_url, get_remote_url, list_remote_branches, parse_git_url,
    switch_branch, RepoInfo,
};
pub use widget_gitbar::{
    render_branch_dropdown, render_branch_input,
    render_dropdown as render_remote_dropdown,
};
