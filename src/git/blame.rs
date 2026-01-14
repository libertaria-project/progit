//! Git Blame - Who broke it and when?
//!
//! Parses `git blame --porcelain` output into structured data.

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use std::collections::HashMap;
use std::process::Command;

/// A single line of blame
#[derive(Debug, Clone)]
pub struct BlameLine {
    pub line_number: usize,
    pub commit_hash: String,
    pub original_line: usize,
    pub author: String,
    pub author_mail: String,
    pub author_time: DateTime<Utc>,
    pub summary: String,
    pub content: String,
}

/// Blame data for a file
#[derive(Debug, Clone)]
pub struct BlameInfo {
    pub file_path: String,
    pub lines: Vec<BlameLine>,
}

impl BlameInfo {
    /// Get blame for a file
    pub fn new(path: &str) -> Result<Self> {
        let output = Command::new("git")
            .args(&["blame", "--porcelain", path])
            .output()
            .context("Failed to run git blame")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "git blame failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let raw = String::from_utf8(output.stdout)?;
        let lines = parse_blame_porcelain(&raw)?;

        Ok(Self {
            file_path: path.to_string(),
            lines,
        })
    }
}

/// Parse git blame --porcelain output
fn parse_blame_porcelain(output: &str) -> Result<Vec<BlameLine>> {
    let mut lines = Vec::new();
    let mut commit_cache: HashMap<String, CommitInfo> = HashMap::new();

    let mut current_commit_hash = String::new();
    let mut current_orig_line = 0;
    let mut current_final_line = 0;

    // Iterate line by line
    for line in output.lines() {
        if line.starts_with('\t') {
            // This is the content line, usually the end of a block for one line of code
            let content = line[1..].to_string();

            // Reconstruct the line info
            if let Some(info) = commit_cache.get(&current_commit_hash) {
                lines.push(BlameLine {
                    line_number: current_final_line,
                    commit_hash: current_commit_hash.clone(),
                    original_line: current_orig_line,
                    author: info.author.clone(),
                    author_mail: info.mail.clone(),
                    author_time: info.time,
                    summary: info.summary.clone(),
                    content,
                });
            } else {
                // This shouldn't happen if porcelain output is standard,
                // because headers come before content
            }
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            // Might be the header line: <sha> <orig_line> <final_line> <group_lines>
            // actually git blame porcelain header format:
            // 40-byte-sha1 <orig_line> <final_line> <num_lines>
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 3 && cols[0].len() == 40 {
                current_commit_hash = cols[0].to_string();
                current_orig_line = cols[1].parse().unwrap_or(0);
                current_final_line = cols[2].parse().unwrap_or(0);

                // If we don't have this commit cached, we expect following headers
                if !commit_cache.contains_key(&current_commit_hash) {
                    commit_cache.insert(current_commit_hash.clone(), CommitInfo::default());
                }
            }
            continue;
        }

        let key = parts[0];
        let value = parts[1];

        // Update cache for the current commit
        if let Some(info) = commit_cache.get_mut(&current_commit_hash) {
            match key {
                "author" => info.author = value.to_string(),
                "author-mail" => info.mail = value.to_string(),
                "author-time" => {
                    let ts: i64 = value.parse().unwrap_or(0);
                    info.time = Utc.timestamp_opt(ts, 0).unwrap();
                }
                "summary" => info.summary = value.to_string(),
                _ => {}
            }
        }
    }

    Ok(lines)
}

#[derive(Default)]
struct CommitInfo {
    author: String,
    mail: String,
    time: DateTime<Utc>,
    summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_blame_porcelain() {
        let output = "464a93d0f04c6303258c70f3f2022e3e506e7804 1 1 2
author Markus Maiwald
author-mail <markus@maiwald.com>
author-time 1709400000
author-tz +0100
committer Markus Maiwald
committer-mail <markus@maiwald.com>
committer-time 1709400000
committer-tz +0100
summary Initial commit
filename src/main.rs
\tuse std::io;
464a93d0f04c6303258c70f3f2022e3e506e7804 2 2
\t
";
        let lines = parse_blame_porcelain(output).unwrap();
        assert_eq!(lines.len(), 2);

        // Line 1
        assert_eq!(lines[0].content, "use std::io;");
        assert_eq!(lines[0].author, "Markus Maiwald");
        assert_eq!(lines[0].line_number, 1);

        // Line 2
        assert_eq!(lines[1].content, "");
        assert_eq!(lines[1].line_number, 2);
    }
}
