// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Fuzzy Command Palette - The "Sublime Text Moment"
//!
//! Global fuzzy search across:
//! - Issues (by title, description, ID)
//! - Commands (TUI actions)
//! - Files (git-tracked files)
//! - Commits (recent commit messages)
//!
//! Target: <200ms to results, <3 keystrokes to execute

use crate::issue::Issue;
use std::cmp::Ordering;

/// Fuzzy search result
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    /// The matched item
    pub item: FuzzyItem,
    /// Match score (higher is better)
    pub score: i32,
    /// Matched character positions for highlighting
    pub positions: Vec<usize>,
}

/// Types of items that can be fuzzy-matched
#[derive(Debug, Clone)]
pub enum FuzzyItem {
    /// Issue match
    Issue {
        id: String,
        title: String,
        status: String,
    },
    /// Command match
    Command {
        name: String,
        description: String,
        action: String,
    },
    /// File match
    File {
        path: String,
        modified: bool,
    },
    /// Commit match
    Commit {
        hash: String,
        message: String,
        author: String,
    },
}

impl FuzzyItem {
    /// Get display text for this item
    pub fn display_text(&self) -> String {
        match self {
            FuzzyItem::Issue { id, title, .. } => format!("#{} {}", &id[..8.min(id.len())], title),
            FuzzyItem::Command { name, description, .. } => format!("{}: {}", name, description),
            FuzzyItem::File { path, modified } => {
                if *modified {
                    format!("* {}", path)
                } else {
                    path.clone()
                }
            }
            FuzzyItem::Commit { hash, message, .. } => {
                format!("{} {}", &hash[..7.min(hash.len())], message)
            }
        }
    }

    /// Get secondary text (subtitle) for this item
    pub fn secondary_text(&self) -> Option<String> {
        match self {
            FuzzyItem::Issue { status, .. } => Some(status.clone()),
            FuzzyItem::Commit { author, .. } => Some(author.clone()),
            _ => None,
        }
    }

    /// Get icon/prefix for this item
    pub fn icon(&self) -> &str {
        match self {
            FuzzyItem::Issue { .. } => "📋",
            FuzzyItem::Command { .. } => "⚡",
            FuzzyItem::File { .. } => "📄",
            FuzzyItem::Commit { .. } => "🔖",
        }
    }
}

/// Fuzzy search engine
pub struct FuzzySearcher {
    /// Cached issue matches
    issues: Vec<FuzzyItem>,
    /// Cached command matches
    commands: Vec<FuzzyItem>,
    /// Cached file matches
    files: Vec<FuzzyItem>,
    /// Cached commit matches
    commits: Vec<FuzzyItem>,
}

impl FuzzySearcher {
    /// Create a new fuzzy searcher
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            commands: Self::build_command_list(),
            files: Vec::new(),
            commits: Vec::new(),
        }
    }

    /// Update issue cache
    pub fn update_issues(&mut self, issues: &[Issue]) {
        self.issues = issues
            .iter()
            .map(|issue| FuzzyItem::Issue {
                id: issue.id.clone(),
                title: issue.title.clone(),
                status: issue.status.as_str().to_string(),
            })
            .collect();
    }

    /// Update file cache from git
    pub fn update_files(&mut self, files: Vec<(String, bool)>) {
        self.files = files
            .into_iter()
            .map(|(path, modified)| FuzzyItem::File { path, modified })
            .collect();
    }

    /// Update commit cache from git
    pub fn update_commits(&mut self, commits: Vec<(String, String, String)>) {
        self.commits = commits
            .into_iter()
            .map(|(hash, message, author)| FuzzyItem::Commit {
                hash,
                message,
                author,
            })
            .collect();
    }

    /// Build static command list
    fn build_command_list() -> Vec<FuzzyItem> {
        vec![
            FuzzyItem::Command {
                name: "New Issue".to_string(),
                description: "Create a new issue".to_string(),
                action: "new_issue".to_string(),
            },
            FuzzyItem::Command {
                name: "Toggle View".to_string(),
                description: "Switch between List and Kanban".to_string(),
                action: "toggle_view".to_string(),
            },
            FuzzyItem::Command {
                name: "Sync".to_string(),
                description: "Sync with remote forge".to_string(),
                action: "sync".to_string(),
            },
            FuzzyItem::Command {
                name: "Search".to_string(),
                description: "Search issues".to_string(),
                action: "search".to_string(),
            },
            FuzzyItem::Command {
                name: "Theme".to_string(),
                description: "Cycle theme".to_string(),
                action: "cycle_theme".to_string(),
            },
            FuzzyItem::Command {
                name: "Settings".to_string(),
                description: "Open settings".to_string(),
                action: "settings".to_string(),
            },
            FuzzyItem::Command {
                name: "Quit".to_string(),
                description: "Exit ProGit".to_string(),
                action: "quit".to_string(),
            },
            FuzzyItem::Command {
                name: "Sort".to_string(),
                description: "Change sort order".to_string(),
                action: "sort".to_string(),
            },
            FuzzyItem::Command {
                name: "Branch".to_string(),
                description: "Manage git branches".to_string(),
                action: "branch".to_string(),
            },
            FuzzyItem::Command {
                name: "Merge Request".to_string(),
                description: "Create or view merge requests".to_string(),
                action: "mr".to_string(),
            },
        ]
    }

    /// Perform fuzzy search across all items
    pub fn search(&self, query: &str) -> Vec<FuzzyMatch> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();

        // Search issues
        for item in &self.issues {
            if let Some(m) = Self::fuzzy_match(query, item) {
                matches.push(m);
            }
        }

        // Search commands
        for item in &self.commands {
            if let Some(m) = Self::fuzzy_match(query, item) {
                matches.push(m);
            }
        }

        // Search files
        for item in &self.files {
            if let Some(m) = Self::fuzzy_match(query, item) {
                matches.push(m);
            }
        }

        // Search commits
        for item in &self.commits {
            if let Some(m) = Self::fuzzy_match(query, item) {
                matches.push(m);
            }
        }

        // Sort by score (descending)
        matches.sort_by(|a, b| b.score.cmp(&a.score));

        // Limit to top 50 results
        matches.truncate(50);

        matches
    }

    /// Fuzzy match a query against an item
    fn fuzzy_match(query: &str, item: &FuzzyItem) -> Option<FuzzyMatch> {
        let text = item.display_text().to_lowercase();
        let query = query.to_lowercase();

        // Simple fuzzy matching algorithm
        let mut score = 0;
        let mut positions = Vec::new();
        let mut query_idx = 0;
        let query_chars: Vec<char> = query.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();

        for (i, &ch) in text_chars.iter().enumerate() {
            if query_idx < query_chars.len() && ch == query_chars[query_idx] {
                positions.push(i);
                query_idx += 1;

                // Bonus for consecutive matches
                if positions.len() > 1 && positions[positions.len() - 1] == positions[positions.len() - 2] + 1 {
                    score += 5;
                }

                // Bonus for start of word
                if i == 0 || text_chars.get(i - 1).map_or(false, |&c| c == ' ' || c == '/' || c == '-') {
                    score += 10;
                }

                score += 1;
            }
        }

        // Must match all query characters
        if query_idx == query_chars.len() {
            // Bonus for exact match
            if text == query {
                score += 100;
            }

            // Bonus for prefix match
            if text.starts_with(&query) {
                score += 50;
            }

            // Penalty for length difference
            score -= (text.len() as i32 - query.len() as i32).abs() / 2;

            Some(FuzzyMatch {
                item: item.clone(),
                score,
                positions,
            })
        } else {
            None
        }
    }
}

impl Default for FuzzySearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact() {
        let item = FuzzyItem::Command {
            name: "New Issue".to_string(),
            description: "Create a new issue".to_string(),
            action: "new_issue".to_string(),
        };

        let result = FuzzySearcher::fuzzy_match("new issue", &item);
        assert!(result.is_some());
        assert!(result.unwrap().score > 50);
    }

    #[test]
    fn test_fuzzy_match_partial() {
        let item = FuzzyItem::Command {
            name: "New Issue".to_string(),
            description: "Create a new issue".to_string(),
            action: "new_issue".to_string(),
        };

        let result = FuzzySearcher::fuzzy_match("ni", &item);
        assert!(result.is_some());
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        let item = FuzzyItem::Command {
            name: "New Issue".to_string(),
            description: "Create a new issue".to_string(),
            action: "new_issue".to_string(),
        };

        let result = FuzzySearcher::fuzzy_match("xyz", &item);
        assert!(result.is_none());
    }

    #[test]
    fn test_search_returns_sorted() {
        let mut searcher = FuzzySearcher::new();
        let results = searcher.search("new");
        
        // Should return results sorted by score
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }
    }
}
