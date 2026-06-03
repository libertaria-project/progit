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
// No ordering needed
use progit_plugin_sdk::contributions::PluginContributionManifest;
use std::collections::HashSet;
use std::path::Path;

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
        /// Source repository name (None = current repo)
        repo: Option<String>,
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
        /// Source repository name (None = current repo)
        repo: Option<String>,
    },
    /// Commit match
    Commit {
        hash: String,
        message: String,
        author: String,
        /// Source repository name (None = current repo)
        repo: Option<String>,
    },
}

impl FuzzyItem {
    /// Get display text for this item
    pub fn display_text(&self) -> String {
        match self {
            FuzzyItem::Issue { id, title, repo, .. } => {
                let prefix = repo.as_ref().map(|r| format!("[{r}] ")).unwrap_or_default();
                format!("{}#{} {}", prefix, &id[..8.min(id.len())], title)
            }
            FuzzyItem::Command {
                name, description, ..
            } => format!("{}: {}", name, description),
            FuzzyItem::File { path, modified, repo } => {
                let prefix = repo.as_ref().map(|r| format!("[{r}] ")).unwrap_or_default();
                if *modified {
                    format!("{}* {}", prefix, path)
                } else {
                    format!("{}{}", prefix, path)
                }
            }
            FuzzyItem::Commit { hash, message, repo, .. } => {
                let prefix = repo.as_ref().map(|r| format!("[{r}] ")).unwrap_or_default();
                format!("{}{} {}", prefix, &hash[..7.min(hash.len())], message)
            }
        }
    }

    /// Get secondary text (subtitle) for this item
    pub fn secondary_text(&self) -> Option<String> {
        match self {
            FuzzyItem::Issue { status, repo, .. } => {
                if let Some(r) = repo {
                    Some(format!("{} in {}", status, r))
                } else {
                    Some(status.clone())
                }
            }
            FuzzyItem::Commit { author, repo, .. } => {
                if let Some(r) = repo {
                    Some(format!("{} in {}", author, r))
                } else {
                    Some(author.clone())
                }
            }
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
    /// Cached issue matches (current repo)
    issues: Vec<FuzzyItem>,
    /// Cached command matches
    commands: Vec<FuzzyItem>,
    /// Cached file matches
    files: Vec<FuzzyItem>,
    /// Cached commit matches
    commits: Vec<FuzzyItem>,
    /// Cached issue matches from nearby repos
    cross_repo_issues: Vec<FuzzyItem>,
}

impl FuzzySearcher {
    /// Create a new fuzzy searcher
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            commands: Self::build_command_list(),
            files: Vec::new(),
            commits: Vec::new(),
            cross_repo_issues: Vec::new(),
        }
    }

    /// Scan parent directories for other git repos and load their issues.
    /// Designed to be fast: max depth 2, max 20 repos, cached after first call.
    pub fn scan_cross_repo_issues(&mut self, base_dir: &std::path::Path, current_repo_name: &str) {
        let mut found = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Walk up to 2 levels of parents from base_dir
        for depth in 0..=2 {
            let base = if depth == 0 {
                base_dir.to_path_buf()
            } else {
                base_dir.ancestors().nth(depth).unwrap_or(base_dir).to_path_buf()
            };

            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    // Skip current repo
                    let repo_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if repo_name == current_repo_name {
                        continue;
                    }
                    // Skip non-git directories
                    if !path.join(".git").exists() {
                        continue;
                    }
                    if !seen.insert(path.clone()) {
                        continue;
                    }

                    // Try to load issues from .project/issues.json
                    let issues_path = path.join(".project").join("issues.json");
                    if let Ok(content) = std::fs::read_to_string(&issues_path) {
                        if let Ok(issues) = serde_json::from_str::<Vec<Issue>>(&content) {
                            for issue in issues {
                                found.push(FuzzyItem::Issue {
                                    id: issue.id,
                                    title: issue.title,
                                    status: issue.status.as_str().to_string(),
                                    repo: Some(repo_name.to_string()),
                                });
                            }
                        }
                    }

                    if found.len() >= 500 {
                        break; // Cap total cross-repo issues
                    }
                }
            }

            if found.len() >= 500 {
                break;
            }
        }

        self.cross_repo_issues = found;
    }

    /// Update issue cache
    pub fn update_issues(&mut self, issues: &[Issue]) {
        self.issues = issues
            .iter()
            .map(|issue| FuzzyItem::Issue {
                id: issue.id.clone(),
                title: issue.title.clone(),
                status: issue.status.as_str().to_string(),
                repo: None,
            })
            .collect();
    }

    /// Update file cache from git
    pub fn update_files(&mut self, files: Vec<(String, bool)>) {
        self.files = files
            .into_iter()
            .map(|(path, modified)| FuzzyItem::File { path, modified, repo: None })
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
                repo: None,
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
            FuzzyItem::Command {
                name: "Project Wiki".to_string(),
                description: "Open repository-owned wiki pages".to_string(),
                action: "project_wiki".to_string(),
            },
            FuzzyItem::Command {
                name: "Project Issues".to_string(),
                description: "Browse repository-owned issue files".to_string(),
                action: "project_issues".to_string(),
            },
            FuzzyItem::Command {
                name: "Plugin Command".to_string(),
                description: "Run an installed plugin command".to_string(),
                action: "plugin_command".to_string(),
            },
            FuzzyItem::Command {
                name: "Sober Doctor".to_string(),
                description: "Run repository governance health checks".to_string(),
                action: "sober_doctor".to_string(),
            },
            FuzzyItem::Command {
                name: "Sober Preflight".to_string(),
                description: "Run deterministic release-gate checks".to_string(),
                action: "sober_preflight".to_string(),
            },
            FuzzyItem::Command {
                name: "Sober Review Preview".to_string(),
                description: "Preview a model review prompt without calling a model".to_string(),
                action: "sober_review_preview".to_string(),
            },
        ]
    }

    /// Update command cache with installed plugin command namespaces.
    pub fn update_plugin_commands(&mut self, repo_root: &Path) {
        let mut commands = Self::build_command_list();
        commands.extend(Self::discover_plugin_commands(repo_root));
        self.commands = commands;
    }

    fn discover_plugin_commands(repo_root: &Path) -> Vec<FuzzyItem> {
        let mut items = Vec::new();
        let mut seen = HashSet::new();

        for plugin_root in [
            repo_root.join("plugins"),
            repo_root.join(".progit").join("plugins"),
        ] {
            let Ok(entries) = std::fs::read_dir(plugin_root) else {
                continue;
            };

            for entry in entries.flatten() {
                let manifest_path = entry.path().join(".progit-plugin.json");
                if !manifest_path.exists() {
                    continue;
                }

                let Ok(raw) = std::fs::read_to_string(&manifest_path) else {
                    continue;
                };
                let Ok(manifest) = PluginContributionManifest::from_json(&raw) else {
                    continue;
                };

                let plugin_name = if manifest.name.is_empty() {
                    entry.file_name().to_string_lossy().to_string()
                } else {
                    manifest.name
                };
                let plugin_description = if manifest.description.is_empty() {
                    "Installed plugin command".to_string()
                } else {
                    manifest.description
                };

                for command in manifest.contributions.commands {
                    if !command.palette {
                        continue;
                    };

                    let command_description = if command.description.is_empty() {
                        plugin_description.clone()
                    } else {
                        command.description.clone()
                    };

                    let command_names = std::iter::once(command.name.as_str())
                        .chain(command.aliases.iter().map(String::as_str));
                    for command_name in command_names {
                        if !seen.insert(command_name.to_string()) {
                            continue;
                        }

                        items.push(FuzzyItem::Command {
                            name: format!("Plugin: {command_name}"),
                            description: format!("{plugin_name}: {command_description}"),
                            action: format!("plugin_command:{command_name}"),
                        });
                    }
                }
            }
        }

        items
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

        // Search cross-repo issues
        for item in &self.cross_repo_issues {
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
                if positions.len() > 1
                    && positions[positions.len() - 1] == positions[positions.len() - 2] + 1
                {
                    score += 5;
                }

                // Bonus for start of word
                if i == 0
                    || text_chars
                        .get(i - 1)
                        .map_or(false, |&c| c == ' ' || c == '/' || c == '-')
                {
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
        let searcher = FuzzySearcher::new();
        let results = searcher.search("new");

        // Should return results sorted by score
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }
    }

    #[test]
    fn test_search_finds_project_view_commands() {
        let searcher = FuzzySearcher::new();

        let wiki = searcher.search("project wiki");
        let issues = searcher.search("project issues");

        assert!(wiki.iter().any(|m| matches!(
            &m.item,
            FuzzyItem::Command { action, .. } if action == "project_wiki"
        )));
        assert!(issues.iter().any(|m| matches!(
            &m.item,
            FuzzyItem::Command { action, .. } if action == "project_issues"
        )));
    }

    #[test]
    fn update_plugin_commands_discovers_manifest_commands() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("plugins").join("sober-raccoon");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join(".progit-plugin.json"),
            r#"{
                "name": "sober-raccoon",
                "description": "Premium Sober governance cockpit",
                "contributions": {
                    "commands": [
                        {
                            "name": "sober",
                            "description": "Run Sober",
                            "args": "passthrough"
                        },
                        {
                            "name": "sober-raccoon",
                            "description": "Open Sober cockpit",
                            "args": "fixed"
                        }
                    ]
                }
            }"#,
        )
        .unwrap();

        let mut searcher = FuzzySearcher::new();
        searcher.update_plugin_commands(dir.path());

        let results = searcher.search("sober");

        assert!(results.iter().any(|m| matches!(
            &m.item,
            FuzzyItem::Command { action, .. } if action == "plugin_command:sober"
        )));
        assert!(results.iter().any(|m| matches!(
            &m.item,
            FuzzyItem::Command { action, .. } if action == "plugin_command:sober-raccoon"
        )));
    }
}
