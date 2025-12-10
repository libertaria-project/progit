//! Issue Operations - CRUD functionality
//!
//! Create, update, delete issues. All operations return new instances.

use super::model::{Issue, Status, Effort};
use chrono::Utc;

/// Update an issue's title
pub fn update_title(issue: &Issue, title: impl Into<String>) -> Issue {
    let mut updated = issue.clone();
    updated.title = title.into();
    updated.updated = Utc::now();
    updated
}

/// Update an issue's description
pub fn update_description(issue: &Issue, description: impl Into<String>) -> Issue {
    let mut updated = issue.clone();
    updated.description = description.into();
    updated.updated = Utc::now();
    updated
}

/// Update an issue's status
pub fn update_status(issue: &Issue, status: Status) -> Issue {
    let mut updated = issue.clone();
    updated.status = status;
    updated.updated = Utc::now();
    updated
}

/// Cycle an issue's status to next state
pub fn cycle_status(issue: &Issue) -> Issue {
    update_status(issue, issue.status.next())
}

/// Update an issue's effort
pub fn update_effort(issue: &Issue, effort: Effort) -> Issue {
    let mut updated = issue.clone();
    updated.effort = effort;
    updated.updated = Utc::now();
    updated
}

/// Update an issue's assignee
pub fn update_assignee(issue: &Issue, assignee: Option<String>) -> Issue {
    let mut updated = issue.clone();
    updated.assignee = assignee;
    updated.updated = Utc::now();
    updated
}

/// Update an issue's sprint
pub fn update_sprint(issue: &Issue, sprint: Option<u32>) -> Issue {
    let mut updated = issue.clone();
    updated.sprint = sprint;
    updated.updated = Utc::now();
    updated
}

/// Update an issue's due date
pub fn update_due(issue: &Issue, due: Option<chrono::DateTime<Utc>>) -> Issue {
    let mut updated = issue.clone();
    updated.due = due;
    updated.updated = Utc::now();
    updated
}

/// Update an issue's blocked status
pub fn update_blocked(issue: &Issue, blocked: bool) -> Issue {
    let mut updated = issue.clone();
    updated.blocked = blocked;
    updated.updated = Utc::now();
    updated
}

/// Toggle blocked status
pub fn toggle_blocked(issue: &Issue) -> Issue {
    update_blocked(issue, !issue.blocked)
}

/// Add a tag to an issue
pub fn add_tag(issue: &Issue, tag: impl Into<String>) -> Issue {
    let mut updated = issue.clone();
    let tag = tag.into();
    if !updated.tags.contains(&tag) {
        updated.tags.push(tag);
    }
    updated.updated = Utc::now();
    updated
}

/// Remove a tag from an issue
pub fn remove_tag(issue: &Issue, tag: &str) -> Issue {
    let mut updated = issue.clone();
    updated.tags.retain(|t| t != tag);
    updated.updated = Utc::now();
    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_title() {
        let issue = Issue::new("Original");
        let updated = update_title(&issue, "Updated");
        assert_eq!(updated.title, "Updated");
        assert!(updated.updated > issue.updated);
    }

    #[test]
    fn test_cycle_status() {
        let issue = Issue::new("Test");
        assert_eq!(issue.status, Status::Backlog);
        
        let cycled = cycle_status(&issue);
        assert_eq!(cycled.status, Status::InProgress);
        
        let cycled2 = cycle_status(&cycled);
        assert_eq!(cycled2.status, Status::Done);
    }

    #[test]
    fn test_add_tag_idempotent() {
        let issue = Issue::new("Test");
        let tagged = add_tag(&issue, "backend");
        let retagged = add_tag(&tagged, "backend");
        assert_eq!(retagged.tags.len(), 1);
    }

    #[test]
    fn test_toggle_blocked() {
        let issue = Issue::new("Test");
        assert!(!issue.blocked);

        let blocked = toggle_blocked(&issue);
        assert!(blocked.blocked);

        let unblocked = toggle_blocked(&blocked);
        assert!(!unblocked.blocked);
    }
}
