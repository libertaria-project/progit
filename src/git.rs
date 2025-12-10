//! Git Feature - Sovereign Index
//!
//! Git repository integration for issue tracking.
//! All git logic lives in `git/` folder.

pub mod repository;
pub mod widget_gitbar;

// Re-export public API
pub use repository::{detect_repo, format_remote_url, refresh_repo, switch_branch, create_branch, delete_branch, get_origin_url, parse_git_url, RemoteInfo, RepoInfo};
pub use widget_gitbar::{render as render_gitbar, render_dropdown as render_remote_dropdown, render_branch_dropdown, render_branch_input};
