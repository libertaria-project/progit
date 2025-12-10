//! Issue Feature - Sovereign Index
//!
//! The atomic unit of work in ProjectsTUI.
//! All issue logic lives in `issue/` folder.

pub mod model;
pub mod operations;
pub mod query;

// Re-export public API
pub use model::{Effort, Issue, Status};
pub use operations::{
    add_tag, cycle_status, remove_tag, update_assignee, update_description, update_effort,
    update_sprint, update_status, update_title,
};
pub use query::{
    completed_effort, filter_blockers, filter_by_sprint, filter_by_status, filter_by_tag,
    group_by_status, search, search_by_title, sort_by_created_desc, sort_by_effort,
    sort_by_updated_desc, total_effort,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api() {
        // Verify all public types are accessible
        let issue = Issue::new("Test");
        assert_eq!(issue.status, Status::Backlog);
        assert_eq!(issue.effort, Effort::Medium);

        let updated = cycle_status(&issue);
        assert_eq!(updated.status, Status::InProgress);
    }
}
