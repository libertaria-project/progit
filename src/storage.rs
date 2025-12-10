//! Storage Feature - Sovereign Index
//!
//! Dual-format persistence: KDL (carbons) + JSON (silicons).
//! All storage logic lives in `storage/` folder.

pub mod json;
pub mod kdl;
pub mod sync;
pub mod config;
pub mod migration;
pub mod cleaner;

// Re-export public API
pub use json::{read_cache, write_cache, IssueCache};
pub use kdl::{issue_filename, parse_kdl, read_all_kdl, read_kdl, serialize_kdl, write_kdl};
pub use sync::{delete_issue, load_issues, save_issue, sync_kdl_to_json};
pub use config::{load_config, save_theme, Config, SyncConfig};
pub use migration::check_and_migrate;
pub use cleaner::cleanup_duplicates;

/// Default paths relative to project root
pub mod paths {
    use std::path::PathBuf;

    pub const PROJECT_DIR: &str = ".project"; // Synced
    pub const LOCAL_DIR: &str = ".progit";   // Ignored

    pub fn issues_dir() -> PathBuf {
        PathBuf::from(PROJECT_DIR).join("issues")
    }
    
    pub fn config_file() -> PathBuf {
        PathBuf::from(PROJECT_DIR).join("config.kdl")
    }
    
    pub fn cache_file() -> PathBuf {
        PathBuf::from(LOCAL_DIR).join("issues.json") // Note: No .cache subdir needed inside .progit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths() {
        assert!(paths::issues_dir().ends_with("issues"));
        assert!(paths::cache_file().ends_with("issues.json"));
    }
}
