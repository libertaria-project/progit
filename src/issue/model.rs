//! Issue Model - Core data structures
//!
//! The atomic unit of work. Immutable by design, transformed via operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Story point effort values (Triangular Sequence)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum Effort {
    Trivial = 1,
    Small = 3,
    Medium = 6,
    Large = 10,
    XLarge = 15,
    Epic = 21,
}

impl TryFrom<u8> for Effort {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Effort::Trivial),
            3 => Ok(Effort::Small),
            6 => Ok(Effort::Medium),
            10 => Ok(Effort::Large),
            15 => Ok(Effort::XLarge),
            21 => Ok(Effort::Epic),
            _ => Err("Invalid effort value. Use: 1, 3, 6, 10, 15, or 21"),
        }
    }
}

impl From<Effort> for u8 {
    fn from(effort: Effort) -> Self {
        effort as u8
    }
}

impl Default for Effort {
    fn default() -> Self {
        Effort::Medium
    }
}

/// Issue workflow status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    #[default]
    Backlog,
    InProgress,
    Done,
}

impl Status {
    /// Cycle to next status (Backlog → InProgress → Done → Backlog)
    pub fn next(self) -> Self {
        match self {
            Status::Backlog => Status::InProgress,
            Status::InProgress => Status::Done,
            Status::Done => Status::Backlog,
        }
    }

    /// Cycle to previous status (Backlog ← InProgress ← Done ← Backlog)
    pub fn prev(self) -> Self {
        match self {
            Status::Backlog => Status::Done,
            Status::InProgress => Status::Backlog,
            Status::Done => Status::InProgress,
        }
    }

    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Backlog => "backlog",
            Status::InProgress => "in-progress",
            Status::Done => "done",
        }
    }
}

/// Default value for updated field (for backward compatibility with old JSON)
fn default_updated() -> DateTime<Utc> {
    Utc::now()
}

/// The atomic unit of work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Unique identifier (UUID v4)
    pub id: String,

    /// Short title (required)
    pub title: String,

    /// Detailed description (optional)
    #[serde(default)]
    pub description: String,

    /// Current workflow status
    #[serde(default)]
    pub status: Status,

    /// Story point estimate
    #[serde(default)]
    pub effort: Effort,

    /// Categorization tags
    #[serde(default)]
    pub tags: Vec<String>,

    /// Assigned team member (optional)
    #[serde(default)]
    pub assignee: Option<String>,

    /// Sprint number (optional)
    #[serde(default)]
    pub sprint: Option<u32>,

    /// Due date for "Time is Over" logic
    #[serde(default)]
    pub due: Option<DateTime<Utc>>,

    /// Started date (when work begins - auto-set on Status::InProgress)
    #[serde(default)]
    pub started: Option<DateTime<Utc>>,

    /// Completed date (when marked Done - auto-set on Status::Done)
    #[serde(default)]
    pub completed: Option<DateTime<Utc>>,

    /// Manually toggled blocker status
    #[serde(default)]
    pub blocked: bool,

    /// Creation timestamp
    pub created: DateTime<Utc>,

    /// Last update timestamp
    #[serde(default = "default_updated")]
    pub updated: DateTime<Utc>,

    /// External references (e.g. "forgejo" -> "42")
    #[serde(default)]
    pub remotes: std::collections::HashMap<String, String>,

    /// Repository ownership (for multi-repo setups)
    /// e.g., "frontend", "backend", "infra"
    #[serde(default)]
    pub repo: Option<String>,
}

impl Issue {
    /// Create a new issue with minimal required fields
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            description: String::new(),
            status: Status::default(),
            effort: Effort::default(),
            tags: Vec::new(),
            assignee: None,
            sprint: None,
            due: None,
            started: None,
            completed: None,
            blocked: false,
            created: now,
            updated: now,
            remotes: std::collections::HashMap::new(),
            repo: None,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Builder: set effort
    pub fn with_effort(mut self, effort: Effort) -> Self {
        self.effort = effort;
        self
    }

    /// Builder: set status
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// Builder: add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: set assignee
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// Builder: set sprint
    pub fn with_sprint(mut self, sprint: u32) -> Self {
        self.sprint = Some(sprint);
        self
    }

    /// Builder: set due date
    pub fn with_due(mut self, due: DateTime<Utc>) -> Self {
        self.due = Some(due);
        self
    }

    /// Builder: set blocked status
    pub fn with_blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    /// Builder: set repository
    pub fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = Some(repo.into());
        self
    }

    /// Check if issue is a blocker (Tag OR Manual toggle)
    pub fn is_blocker(&self) -> bool {
        self.blocked || self.tags.iter().any(|t| t.eq_ignore_ascii_case("blocker"))
    }

    /// Check if issue is overdue (Time is Over)
    pub fn is_overdue(&self) -> bool {
        if self.status == Status::Done {
            return false;
        }
        if let Some(due) = self.due {
            Utc::now() > due
        } else {
            false
        }
    }

    /// Get short ID (first 8 chars)
    pub fn short_id(&self) -> &str {
        &self.id[..8.min(self.id.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_creation() {
        let issue = Issue::new("Test issue");
        assert_eq!(issue.title, "Test issue");
        assert_eq!(issue.status, Status::Backlog);
        assert_eq!(issue.effort, Effort::Medium);
        assert!(!issue.id.is_empty());
    }

    #[test]
    fn test_status_cycle() {
        assert_eq!(Status::Backlog.next(), Status::InProgress);
        assert_eq!(Status::InProgress.next(), Status::Done);
        assert_eq!(Status::Done.next(), Status::Backlog);
    }

    #[test]
    fn test_effort_from_u8() {
        // Triangular sequence test
        assert_eq!(Effort::try_from(10).unwrap(), Effort::Large); // Was 5, now 10
        assert!(Effort::try_from(5).is_err()); // 5 is no longer valid
    }

    #[test]
    fn test_blocker_detection() {
        let issue = Issue::new("Blocked").with_tags(vec!["Blocker".to_string()]);
        assert!(issue.is_blocker());

        let manual_block = Issue::new("Manual Block").with_blocked(true);
        assert!(manual_block.is_blocker());

        let normal = Issue::new("Normal");
        assert!(!normal.is_blocker());
    }

    #[test]
    fn test_overdue_detection() {
        let past = Utc::now() - chrono::Duration::days(1);
        let future = Utc::now() + chrono::Duration::days(1);

        let overdue = Issue::new("Overdue").with_due(past);
        assert!(overdue.is_overdue());

        let on_time = Issue::new("On Time").with_due(future);
        assert!(!on_time.is_overdue());

        let done_late = Issue::new("Done Late")
            .with_due(past)
            .with_status(Status::Done);
        assert!(!done_late.is_overdue()); // Done issues are never overdue
    }
}
