//! JSON Storage - Machine-fast cache
//!
//! Read and write JSON cache for silicon-based processors.

use crate::issue::Issue;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// JSON cache structure
#[derive(Debug, Serialize, Deserialize)]
pub struct IssueCache {
    pub issues: Vec<Issue>,
    pub meta: CacheMeta,
}

/// Cache metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheMeta {
    pub last_sync: DateTime<Utc>,
    pub version: u32,
}

impl Default for IssueCache {
    fn default() -> Self {
        Self {
            issues: Vec::new(),
            meta: CacheMeta {
                last_sync: Utc::now(),
                version: 1,
            },
        }
    }
}

/// Read the JSON cache
pub fn read_cache(path: &Path) -> Result<IssueCache> {
    if !path.exists() {
        return Ok(IssueCache::default());
    }

    let content = fs::read_to_string(path).context("Failed to read JSON cache")?;
    let cache: IssueCache = serde_json::from_str(&content).context("Failed to parse JSON cache")?;
    Ok(cache)
}

/// Write the JSON cache
pub fn write_cache(issues: &[Issue], path: &Path) -> Result<()> {
    let cache = IssueCache {
        issues: issues.to_vec(),
        meta: CacheMeta {
            last_sync: Utc::now(),
            version: 1,
        },
    };

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&cache).context("Failed to serialize JSON")?;
    fs::write(path, json).context("Failed to write JSON cache")?;
    Ok(())
}

/// Check if cache is stale compared to a timestamp
pub fn is_cache_stale(path: &Path, since: DateTime<Utc>) -> bool {
    match read_cache(path) {
        Ok(cache) => cache.meta.last_sync < since,
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_roundtrip() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("issues.json");

        let issues = vec![
            Issue::new("Issue 1"),
            Issue::new("Issue 2"),
        ];

        write_cache(&issues, &cache_path).unwrap();
        let loaded = read_cache(&cache_path).unwrap();

        assert_eq!(loaded.issues.len(), 2);
        assert_eq!(loaded.issues[0].title, "Issue 1");
    }

    #[test]
    fn test_missing_cache_returns_default() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("nonexistent.json");

        let cache = read_cache(&cache_path).unwrap();
        assert!(cache.issues.is_empty());
    }
}
