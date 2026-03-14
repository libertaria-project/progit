//! Issue Feature - Sovereign Index
//!
//! The atomic unit of work in ProjectsTUI.
//! All issue logic lives in `issue/` folder.

pub mod model;
pub mod operations;
pub mod query;

// Re-export public API
pub use model::{Effort, Issue, Status};

#[cfg(test)]
mod tests {
    use super::*;
    use super::operations::cycle_status;

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
