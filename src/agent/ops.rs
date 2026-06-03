// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Agent Operations
//!
//! Handles applying changes proposed by the AI agent.

use crate::virtual_branch::VirtualBranchManager;
use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Apply a patch provided by the agent and assign resulting hunks to the branch
pub fn apply_agent_patch(
    manager: &mut VirtualBranchManager,
    branch_id: &str,
    patch_content: &str,
) -> Result<usize> {
    // 1. Sanitize patch content
    // Agents often wrap code in markdown ```diff ... ```
    let clean_patch = extract_diff_content(patch_content);

    // 2. Write to temp file
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(clean_patch.as_bytes())?;

    // 3. Apply patch using git apply
    // usage: git apply --cached? No, working dir.
    // We use --unidiff-zero to match context leniently if needed,
    // but standard apply is safer.
    let status = Command::new("git")
        .arg("apply")
        .arg("--verbose")
        .arg(temp_file.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .context("Failed to execute git apply")?;

    if !status.success() {
        return Err(anyhow!("Failed to apply patch. Please check conflicts."));
    }

    // 4. Detect new hunks
    let new_hunks = manager.detect_workspace_hunks()?;

    // 5. Assign to branch if unowned
    let mut assigned_count = 0;
    if let Some(branch) = manager.get_mut(branch_id) {
        for hunk in new_hunks {
            // Check if this hunk is already owned by another branch?
            // For now, simpler: just assign it.
            // In a real system we'd check conflicts.
            if !branch.owned_hunks.contains(&hunk) {
                branch.assign_hunk(hunk);
                assigned_count += 1;
            }
        }
        // Save the branch state
        // We need to save via manager, but we have mutable borrow on branch.
        // So we just modified the branch in memory.
    }

    // Save the specific branch to disk
    manager.save_branch(branch_id)?;

    Ok(assigned_count)
}

/// Extract content from markdown code blocks if present
fn extract_diff_content(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_block = false;
    let mut extracted = Vec::new();

    // Check if it's explicitly wrapped
    let has_code_block = content.contains("```diff") || content.contains("```patch");

    if !has_code_block {
        // Assume raw diff if it looks like one
        return content.to_string();
    }

    for line in lines {
        if line.trim().starts_with("```diff") || line.trim().starts_with("```patch") {
            in_block = true;
            continue;
        }
        if line.trim().starts_with("```") && in_block {
            in_block = false;
            continue;
        }

        if in_block {
            extracted.push(line);
        }
    }

    extracted.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_diff() {
        let input = "Here is the fix:\n\n```diff\n--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-fn foo()\n+fn bar()\n```\n\nHope that helps!";
        let extracted = extract_diff_content(input);
        assert!(extracted.contains("--- a/file.rs"));
        assert!(!extracted.contains("Here is the fix"));
        assert!(!extracted.contains("```diff"));
    }
}
