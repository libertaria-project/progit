// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Highlight result cache, keyed by (language, content) blake3 hash.
//!
//! [PERF] Calling Lua per diff-line per frame is unworkable. Real-world
//! diffs render at 60 Hz on user scroll/keypress; the same line-content
//! is rendered hundreds of times. We cache by content hash so the Lua
//! roundtrip happens once per unique line, not once per frame.
//!
//! Eviction is bulk-clear at MAX_ENTRIES rather than LRU because:
//! 1. The cost of re-warming a few hundred line entries is invisible at
//!    terminal frame rates (sub-ms).
//! 2. LRU adds a doubly-linked list, an extra index, and roughly
//!    triples memory per entry.
//! 3. The cache is per-process, lives only while ProGit runs, and is
//!    rebuilt cheaply on next render.

use blake3::Hasher;
use progit_plugin_sdk::render::HighlightResponse;
use std::collections::HashMap;

const MAX_ENTRIES: usize = 4096;

/// In-memory highlight cache.
#[derive(Default)]
pub struct HighlightCache {
    entries: HashMap<u64, HighlightResponse>,
    hits: u64,
    misses: u64,
}

impl HighlightCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a cached response. Updates hit/miss counters.
    pub fn get(&mut self, key: u64) -> Option<HighlightResponse> {
        if let Some(v) = self.entries.get(&key) {
            self.hits += 1;
            Some(v.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store a response. Bulk-evicts when full.
    pub fn insert(&mut self, key: u64, value: HighlightResponse) {
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.clear();
        }
        self.entries.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn hit_rate(&self) -> Option<f64> {
        let total = self.hits + self.misses;
        if total == 0 {
            None
        } else {
            Some(self.hits as f64 / total as f64)
        }
    }
}

/// Compute the cache key for a (language, content) pair.
///
/// blake3 over `language || \0 || content`, truncated to u64. Collisions
/// are vanishingly unlikely at the scales we care about (a single
/// session's worth of diff lines).
pub fn key_for(language: Option<&str>, content: &str) -> u64 {
    let mut h = Hasher::new();
    if let Some(l) = language {
        h.update(l.as_bytes());
    }
    h.update(b"\0");
    h.update(content.as_bytes());
    let hash = h.finalize();
    let b = hash.as_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use progit_plugin_sdk::render::TokenSpan;

    fn fake_response() -> HighlightResponse {
        HighlightResponse {
            spans: vec![TokenSpan::plain("x")],
        }
    }

    #[test]
    fn roundtrip() {
        let mut c = HighlightCache::new();
        let k = key_for(Some("rust"), "fn main() {}");
        assert!(c.get(k).is_none());
        c.insert(k, fake_response());
        assert!(c.get(k).is_some());
    }

    #[test]
    fn lang_changes_key() {
        let k_rust = key_for(Some("rust"), "x");
        let k_py = key_for(Some("python"), "x");
        let k_none = key_for(None, "x");
        assert_ne!(k_rust, k_py);
        assert_ne!(k_rust, k_none);
        assert_ne!(k_py, k_none);
    }

    #[test]
    fn bulk_eviction_at_cap() {
        let mut c = HighlightCache::new();
        for i in 0..(MAX_ENTRIES + 5) {
            c.insert(i as u64, fake_response());
        }
        // After eviction we should hold only the post-eviction inserts.
        assert!(c.len() < MAX_ENTRIES);
    }

    #[test]
    fn hit_rate_tracking() {
        let mut c = HighlightCache::new();
        let k = key_for(None, "x");
        assert!(c.hit_rate().is_none());
        c.get(k); // miss
        c.insert(k, fake_response());
        c.get(k); // hit
        c.get(k); // hit
        let rate = c.hit_rate().unwrap();
        assert!((rate - 2.0 / 3.0).abs() < 1e-9, "got {}", rate);
    }
}
