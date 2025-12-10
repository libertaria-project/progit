//! KDL Storage - Human-readable issue persistence
//!
//! Read and write KDL files for carbon-based lifeforms.

use crate::issue::{Effort, Issue, Status};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};
use std::fs;
use std::path::Path;

/// Read all issues from a KDL directory
pub fn read_all_kdl(dir: &Path) -> Result<Vec<Issue>> {
    let mut issues = Vec::new();

    if !dir.exists() {
        return Ok(issues);
    }

    for entry in fs::read_dir(dir).context("Failed to read issues directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "kdl") {
            let issue = read_kdl(&path)?;
            issues.push(issue);
        }
    }

    Ok(issues)
}

/// Read a single issue from a KDL file
pub fn read_kdl(path: &Path) -> Result<Issue> {
    let content = fs::read_to_string(path).context("Failed to read KDL file")?;
    parse_kdl(&content)
}

/// Parse KDL content into an Issue
pub fn parse_kdl(content: &str) -> Result<Issue> {
    let doc: KdlDocument = content.parse().context("Failed to parse KDL")?;

    // Find the issue node
    let issue_node = doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "issue")
        .context("No 'issue' node found")?;

    // Extract ID from entry
    let id = issue_node
        .get("id")
        .and_then(|e| e.value().as_string())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Parse children
    let children = issue_node.children().map(|c| c.nodes()).unwrap_or(&[]);

    let title = get_string_value(children, "title").unwrap_or_default();
    let description = get_string_value(children, "description").unwrap_or_default();
    let status = get_string_value(children, "status")
        .map(|s| parse_status(&s))
        .unwrap_or_default();
    let effort = get_int_value(children, "effort")
        .and_then(|e| Effort::try_from(e as u8).ok())
        .unwrap_or_default();
    let assignee = get_string_value(children, "assignee");
    let sprint = get_int_value(children, "sprint").map(|s| s as u32);
    let due = get_datetime_value(children, "due");
    let started = get_datetime_value(children, "started");
    let completed = get_datetime_value(children, "completed");
    let blocked = get_bool_value(children, "blocked").unwrap_or(false);
    let tags = get_tags(children);
    let created = get_datetime_value(children, "created").unwrap_or_else(Utc::now);
    let updated = get_datetime_value(children, "updated").unwrap_or_else(Utc::now);
    let remotes = get_remotes(children);
    let repo = get_string_value(children, "repo");

    Ok(Issue {
        id,
        title,
        description,
        status,
        effort,
        tags,
        assignee,
        sprint,
        due,
        started,
        completed,
        blocked,
        created,
        updated,
        remotes,
        repo,
    })
}

/// Write an issue to a KDL file
pub fn write_kdl(issue: &Issue, path: &Path) -> Result<()> {
    let content = serialize_kdl(issue);

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content).context("Failed to write KDL file")?;
    Ok(())
}

/// Serialize an Issue to KDL format
pub fn serialize_kdl(issue: &Issue) -> String {
    let mut lines = Vec::new();

    lines.push(format!("issue id=\"{}\" {{", issue.id));
    lines.push(format!("    title \"{}\"", escape_kdl_string(&issue.title)));

    if !issue.description.is_empty() {
        lines.push(format!(
            "    description \"{}\"",
            escape_kdl_string(&issue.description)
        ));
    }

    lines.push(format!("    status \"{}\"", issue.status.as_str()));
    lines.push(format!("    effort {}", issue.effort as u8));

    if let Some(ref assignee) = issue.assignee {
        lines.push(format!("    assignee \"{}\"", assignee));
    }

    if let Some(sprint) = issue.sprint {
        lines.push(format!("    sprint {}", sprint));
    }

    if let Some(due) = issue.due {
        lines.push(format!("    due \"{}\"", due.to_rfc3339()));
    }

    if let Some(started) = issue.started {
        lines.push(format!("    started \"{}\"", started.to_rfc3339()));
    }

    if let Some(completed) = issue.completed {
        lines.push(format!("    completed \"{}\"", completed.to_rfc3339()));
    }

    if issue.blocked {
        lines.push("    blocked true".to_string());
    }

    if !issue.tags.is_empty() {
        lines.push("    tags {".to_string());
        for tag in &issue.tags {
            lines.push(format!("        - \"{}\"", tag));
        }
        lines.push("    }".to_string());
    }

    if !issue.remotes.is_empty() {
        lines.push("    remotes {".to_string());
        for (provider, remote_id) in &issue.remotes {
            lines.push(format!("        {} \"{}\"", provider, remote_id));
        }
        lines.push("    }".to_string());
    }
    
    if let Some(ref repo) = issue.repo {
        lines.push(format!("    repo \"{}\"", repo));
    }

    lines.push(format!("    created \"{}\"", issue.created.to_rfc3339()));
    lines.push(format!("    updated \"{}\"", issue.updated.to_rfc3339()));

    lines.push("}".to_string());

    lines.join("\n")
}

/// Generate a filename for an issue
pub fn issue_filename(issue: &Issue) -> String {
    let slug: String = issue
        .title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(5)
        .collect::<Vec<_>>()
        .join("-");

    format!("{}-{}.kdl", &issue.id[..8.min(issue.id.len())], slug)
}

// Helper functions

fn get_string_value(nodes: &[KdlNode], name: &str) -> Option<String> {
    nodes
        .iter()
        .find(|n| n.name().value() == name)
        .and_then(|n| n.entries().first())
        .and_then(|e| e.value().as_string())
        .map(|s| s.to_string())
}

fn get_int_value(nodes: &[KdlNode], name: &str) -> Option<i64> {
    nodes
        .iter()
        .find(|n| n.name().value() == name)
        .and_then(|n| n.entries().first())
        .and_then(|e| e.value().as_i64())
}

fn get_datetime_value(nodes: &[KdlNode], name: &str) -> Option<DateTime<Utc>> {
    get_string_value(nodes, name).and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)))
}

fn get_bool_value(nodes: &[KdlNode], name: &str) -> Option<bool> {
    nodes
        .iter()
        .find(|n| n.name().value() == name)
        .and_then(|n| n.entries().first())
        .and_then(|e| e.value().as_bool())
}

fn get_tags(nodes: &[KdlNode]) -> Vec<String> {
    nodes
        .iter()
        .find(|n| n.name().value() == "tags")
        .and_then(|n| n.children())
        .map(|children| {
            children
                .nodes()
                .iter()
                .filter(|n| n.name().value() == "-")
                .filter_map(|n| n.entries().first())
                .filter_map(|e| e.value().as_string())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn get_remotes(nodes: &[KdlNode]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    
    if let Some(remotes_node) = nodes.iter().find(|n| n.name().value() == "remotes") {
        if let Some(children) = remotes_node.children() {
            for node in children.nodes() {
                let key = node.name().value().to_string();
                if let Some(val) = node.entries().first().and_then(|e| e.value().as_string()) {
                    map.insert(key, val.to_string());
                }
            }
        }
    }
    
    map
}

fn parse_status(s: &str) -> Status {
    match s.to_lowercase().as_str() {
        "in-progress" | "inprogress" => Status::InProgress,
        "done" | "completed" => Status::Done,
        _ => Status::Backlog,
    }
}

fn escape_kdl_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kdl() {
        let content = r#"
issue id="test-123" {
    title "Fix the bug"
    status "in-progress"
    effort 5
    tags {
        - "backend"
        - "blocker"
    }
    blocked true
}
}
"#;
        let issue = parse_kdl(content).unwrap();
        assert_eq!(issue.id, "test-123");
        assert_eq!(issue.title, "Fix the bug");
        assert_eq!(issue.status, Status::InProgress);
        assert_eq!(issue.status, Status::InProgress);
        assert_eq!(issue.effort, Effort::Large);
        assert_eq!(issue.tags, vec!["backend", "blocker"]);
        assert!(issue.blocked);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let issue = Issue::new("Test issue")
            .with_status(Status::InProgress)
            .with_effort(Effort::Large)
            .with_tags(vec!["test".to_string()])
            .with_blocked(true);

        let kdl = serialize_kdl(&issue);
        let parsed = parse_kdl(&kdl).unwrap();

        assert_eq!(parsed.title, issue.title);
        assert_eq!(parsed.status, issue.status);
        assert_eq!(parsed.effort, issue.effort);
        assert_eq!(parsed.blocked, issue.blocked);
    }

    #[test]
    fn test_issue_filename() {
        let issue = Issue::new("Fix the authentication bug!");
        let filename = issue_filename(&issue);
        assert!(filename.ends_with(".kdl"));
        assert!(filename.contains("fix-the-authentication-bug"));
    }
}
