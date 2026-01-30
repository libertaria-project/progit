//! Merge Request Model
//!
//! Core data structures for representing merge/pull requests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Merge Request state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MRState {
    /// Open and ready for review
    Open,
    /// Merged successfully
    Merged,
    /// Closed without merging
    Closed,
    /// Draft/WIP state
    Draft,
}

impl MRState {
    pub fn as_str(&self) -> &'static str {
        match self {
            MRState::Open => "open",
            MRState::Merged => "merged",
            MRState::Closed => "closed",
            MRState::Draft => "draft",
        }
    }
}

/// Merge Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequest {
    /// Local ID (generated)
    pub id: String,

    /// Remote MR number (GitLab IID, GitHub PR number)
    pub remote_id: Option<u64>,

    /// Source branch
    pub source_branch: String,

    /// Target branch (usually main/master)
    pub target_branch: String,

    /// MR title
    pub title: String,

    /// Description/body
    pub description: String,

    /// Current state
    pub state: MRState,

    /// Author username
    pub author: Option<String>,

    /// Assignees
    pub assignees: Vec<String>,

    /// Labels/tags
    pub labels: Vec<String>,

    /// Linked issue ID(s)
    pub linked_issues: Vec<String>,

    /// Remote URL (web link)
    pub web_url: Option<String>,

    /// Created timestamp
    pub created: DateTime<Utc>,

    /// Updated timestamp
    pub updated: DateTime<Utc>,

    /// Merge timestamp (if merged)
    pub merged_at: Option<DateTime<Utc>>,

    /// Draft/WIP flag
    pub is_draft: bool,

    /// Review Stats (Approvals)
    pub approvals: u32,
    pub upvotes: u32,
    pub downvotes: u32,

    /// CI/CD Pipeline status (from plugin)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_status: Option<String>, // "passed"|"failed"|"running"|"pending"
}

impl MergeRequest {
    /// Create a new MR with smart defaults
    pub fn new(source_branch: &str, target_branch: &str, title: &str) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            remote_id: None,
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            title: title.to_string(),
            description: String::new(),
            state: MRState::Open,
            author: None,
            assignees: Vec::new(),
            labels: Vec::new(),
            linked_issues: Vec::new(),
            web_url: None,
            created: now,
            updated: now,
            merged_at: None,
            is_draft: false,
            approvals: 0,
            upvotes: 0,
            downvotes: 0,
            pipeline_status: None,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Builder: link to issue
    pub fn link_issue(mut self, issue_id: impl Into<String>) -> Self {
        self.linked_issues.push(issue_id.into());
        self
    }

    /// Builder: add assignee
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignees.push(assignee.into());
        self
    }

    /// Builder: add labels
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Builder: mark as draft
    pub fn as_draft(mut self) -> Self {
        self.is_draft = true;
        self.state = MRState::Draft;
        self
    }

    /// Short ID for display (first 8 chars)
    pub fn short_id(&self) -> String {
        self.id.chars().take(8).collect()
    }

    /// Display name: remote ID if available, otherwise short local ID
    pub fn display_id(&self) -> String {
        if let Some(remote) = self.remote_id {
            format!("!{}", remote)
        } else {
            format!("#{}", self.short_id())
        }
    }
}
