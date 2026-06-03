// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Agent Context Gathering
//!
//! Gathers relevant code context from the virtual branch to build the LLM prompt.

use crate::virtual_branch::VirtualBranch;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

/// Context gathered for the agent
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub files: Vec<FileContext>,
}

/// Content of a single file
#[derive(Debug, Clone)]
pub struct FileContext {
    pub path: String,
    pub content: String,
}

impl AgentContext {
    /// Format context as an XML-like string for the prompt
    pub fn to_prompt_string(&self) -> String {
        let mut out = String::new();
        out.push_str("Here are the relevant files for your context:\n\n");

        for file in &self.files {
            out.push_str(&format!("<file path=\"{}\">\n", file.path));
            out.push_str(&file.content);
            out.push_str("\n</file>\n\n");
        }

        out
    }
}

/// Gather context from a virtual branch
pub fn gather_context(branch: &VirtualBranch, repo_root: &Path) -> Result<AgentContext> {
    // 1. Identify unique files
    let mut file_paths = HashSet::new();

    for hunk in &branch.owned_hunks {
        file_paths.insert(hunk.file_path.clone());
    }
    for hunk in &branch.staged_hunks {
        file_paths.insert(hunk.file_path.clone());
    }

    // 2. Read contents
    let mut files = Vec::new();
    for path_str in file_paths {
        let abs_path = repo_root.join(&path_str);
        if abs_path.exists() {
            let content = std::fs::read_to_string(&abs_path)
                .with_context(|| format!("Failed to read context file: {}", path_str))?;

            files.push(FileContext {
                path: path_str,
                content,
            });
        }
    }

    // Sort for determinism
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(AgentContext { files })
}
