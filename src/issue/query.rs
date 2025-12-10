//! Issue Query - Filtering and search operations
//!
//! Pure functions for querying issue collections.

use super::model::{Issue, Status};

/// Filter issues by status
pub fn filter_by_status(issues: &[Issue], status: Status) -> Vec<&Issue> {
    issues.iter().filter(|i| i.status == status).collect()
}

/// Filter issues by sprint number
pub fn filter_by_sprint(issues: &[Issue], sprint: u32) -> Vec<&Issue> {
    issues.iter().filter(|i| i.sprint == Some(sprint)).collect()
}

/// Filter issues by tag
pub fn filter_by_tag<'a>(issues: &'a [Issue], tag: &str) -> Vec<&'a Issue> {
    issues
        .iter()
        .filter(|i| i.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
        .collect()
}

/// Filter blocker issues
pub fn filter_blockers(issues: &[Issue]) -> Vec<&Issue> {
    issues.iter().filter(|i| i.is_blocker()).collect()
}

/// Search issues by title (case-insensitive substring match)
pub fn search_by_title<'a>(issues: &'a [Issue], query: &str) -> Vec<&'a Issue> {
    let query_lower = query.to_lowercase();
    issues
        .iter()
        .filter(|i| i.title.to_lowercase().contains(&query_lower))
        .collect()
}

/// Search issues by title or description
pub fn search<'a>(issues: &'a [Issue], query: &str) -> Vec<&'a Issue> {
    let query_lower = query.to_lowercase();
    issues
        .iter()
        .filter(|i| {
            i.title.to_lowercase().contains(&query_lower)
                || i.description.to_lowercase().contains(&query_lower)
        })
        .collect()
}

/// Sort issues by effort (ascending)
pub fn sort_by_effort(issues: &mut [Issue]) {
    issues.sort_by_key(|i| i.effort as u8);
}

/// Sort issues by creation date (newest first)
pub fn sort_by_created_desc(issues: &mut [Issue]) {
    issues.sort_by(|a, b| b.created.cmp(&a.created));
}

/// Sort issues by update date (newest first)
pub fn sort_by_updated_desc(issues: &mut [Issue]) {
    issues.sort_by(|a, b| b.updated.cmp(&a.updated));
}

/// Group issues by status (returns HashMap alternative: tuple vecs)
pub fn group_by_status(issues: &[Issue]) -> (Vec<&Issue>, Vec<&Issue>, Vec<&Issue>) {
    let backlog = filter_by_status(issues, Status::Backlog);
    let in_progress = filter_by_status(issues, Status::InProgress);
    let done = filter_by_status(issues, Status::Done);
    (backlog, in_progress, done)
}

/// Calculate total effort points
pub fn total_effort(issues: &[Issue]) -> u32 {
    issues.iter().map(|i| i.effort as u32).sum()
}

/// Calculate completed effort points
pub fn completed_effort(issues: &[Issue]) -> u32 {
    issues
        .iter()
        .filter(|i| i.status == Status::Done)
        .map(|i| i.effort as u32)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::model::Effort;

    fn sample_issues() -> Vec<Issue> {
        vec![
            Issue::new("Fix auth")
                .with_status(Status::InProgress)
                .with_effort(Effort::Large)
                .with_tags(vec!["backend".to_string(), "blocker".to_string()]),
            Issue::new("Add dashboard")
                .with_status(Status::Backlog)
                .with_effort(Effort::XLarge)
                .with_sprint(1),
            Issue::new("Write docs")
                .with_status(Status::Done)
                .with_effort(Effort::Small),
        ]
    }

    #[test]
    fn test_filter_by_status() {
        let issues = sample_issues();
        let backlog = filter_by_status(&issues, Status::Backlog);
        assert_eq!(backlog.len(), 1);
        assert_eq!(backlog[0].title, "Add dashboard");
    }

    #[test]
    fn test_filter_blockers() {
        let issues = sample_issues();
        let blockers = filter_blockers(&issues);
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].title, "Fix auth");
    }

    #[test]
    fn test_search() {
        let issues = sample_issues();
        let results = search_by_title(&issues, "AUTH");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_total_effort() {
        let issues = sample_issues();
        // 5 (Large) + 8 (XLarge) + 2 (Small) = 15
        assert_eq!(total_effort(&issues), 15);
    }

    #[test]
    fn test_completed_effort() {
        let issues = sample_issues();
        // Only "Write docs" (Small=2) is Done
        assert_eq!(completed_effort(&issues), 2);
    }
}
