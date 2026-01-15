// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Virtual Branch Management
//!
//! [ARCH] Virtual branches allow multiple branches to be "applied" to the same
//! working directory simultaneously. Each branch owns a set of hunks, enabling
//! parallel development without context switching.
//!
//! Key concepts:
//! - **Lane**: A virtual branch displayed as a column in the TUI
//! - **Hunk Ownership**: Each changed hunk belongs to exactly one virtual branch
//! - **Per-Branch Staging**: Each branch has its own staging area
//!
//! Storage: JSON files in `.project/branches/`

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Virtual branch state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualBranch {
    /// Unique identifier
    pub id: String,
    /// User-facing name
    pub name: String,
    /// Base commit SHA where this branch started
    pub base_commit: String,
    /// Last committed state (None = uncommitted changes only)
    pub head_commit: Option<String>,
    /// Is this branch "applied" (visible in working directory)?
    pub applied: bool,
    /// Branch order in the lanes view (0 = leftmost)
    pub order: u32,
    /// Hunks owned by this branch
    pub owned_hunks: Vec<HunkRef>,
    /// Hunks staged for next commit
    pub staged_hunks: Vec<HunkRef>,
    /// Associated AI agent session (if any)
    pub agent_session: Option<AgentSession>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,
    /// Upstream branch name (for push/pull)
    pub upstream: Option<String>,
    /// Has conflicted commits after rebase?
    pub has_conflicts: bool,
}

/// Reference to a specific hunk within a file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HunkRef {
    /// File path relative to repo root
    pub file_path: String,
    /// Content-based hash for hunk identity
    pub hash: String,
    /// Starting line in old file
    pub old_start: u32,
    /// Number of lines in old file
    pub old_count: u32,
    /// Starting line in new file
    pub new_start: u32,
    /// Number of lines in new file
    pub new_count: u32,
}

impl HunkRef {
    /// Check if this hunk overlaps with another hunk in the same file
    pub fn overlaps_with(&self, other: &HunkRef) -> bool {
        if self.file_path != other.file_path {
            return false;
        }
        
        // Check if line ranges overlap
        let self_start = self.new_start;
        let self_end = self.new_start + self.new_count;
        let other_start = other.new_start;
        let other_end = other.new_start + other.new_count;
        
        // Ranges overlap if one starts before the other ends
        self_start < other_end && other_start < self_end
    }
}

impl HunkRef {
    /// Create a new HunkRef with computed hash
    pub fn new(
        file_path: impl Into<String>,
        content: &str,
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            hash: compute_hash(content),
            old_start,
            old_count,
            new_start,
            new_count,
        }
    }
}

/// AI Agent session associated with a virtual branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Session identifier
    pub id: String,
    /// LLM model used (e.g., "ollama:llama3", "claude-3-opus")
    pub model: String,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Total input tokens consumed
    pub input_tokens: u64,
    /// Total output tokens generated
    pub output_tokens: u64,
    /// Estimated cost in USD
    pub cost_usd: f64,
    /// Prompt history (last N prompts for context)
    pub prompts: Vec<AgentPrompt>,
    /// Current status
    pub status: AgentStatus,
    /// Auto-commit after task completion?
    pub auto_commit: bool,
    /// YOLO mode - skip all permission prompts
    pub yolo_mode: bool,
}

/// A single prompt/response pair in an agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPrompt {
    /// User's prompt text
    pub prompt: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Token count for this exchange
    pub tokens: u64,
    /// Files modified during this prompt
    pub files_modified: Vec<String>,
}

/// Agent execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    /// Ready for new prompts
    Idle,
    /// Thinking/planning
    Thinking { task: String },
    /// Executing a tool call
    Executing { tool: String },
    /// Waiting for user permission
    AwaitingPermission { action: String },
    /// Completed task
    Completed { summary: String },
    /// Error state
    Error { message: String },
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl VirtualBranch {
    /// Create a new virtual branch
    pub fn new(name: impl Into<String>, base_commit: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            base_commit: base_commit.into(),
            head_commit: None,
            applied: true,
            order: 0,
            owned_hunks: Vec::new(),
            staged_hunks: Vec::new(),
            agent_session: None,
            created_at: now,
            updated_at: now,
            upstream: None,
            has_conflicts: false,
        }
    }

    /// Assign a hunk to this branch
    pub fn assign_hunk(&mut self, hunk: HunkRef) {
        if !self.owned_hunks.contains(&hunk) {
            self.owned_hunks.push(hunk);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a hunk from this branch
    pub fn remove_hunk(&mut self, hunk: &HunkRef) -> bool {
        let before = self.owned_hunks.len();
        self.owned_hunks.retain(|h| h != hunk);
        self.staged_hunks.retain(|h| h != hunk);
        if self.owned_hunks.len() != before {
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Stage a hunk for commit
    pub fn stage_hunk(&mut self, hunk: &HunkRef) -> bool {
        if self.owned_hunks.contains(hunk) && !self.staged_hunks.contains(hunk) {
            self.staged_hunks.push(hunk.clone());
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Unstage a hunk
    pub fn unstage_hunk(&mut self, hunk: &HunkRef) -> bool {
        let before = self.staged_hunks.len();
        self.staged_hunks.retain(|h| h != hunk);
        if self.staged_hunks.len() != before {
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Start an AI agent session
    pub fn start_agent_session(&mut self, model: impl Into<String>) {
        self.agent_session = Some(AgentSession {
            id: Uuid::new_v4().to_string(),
            model: model.into(),
            started_at: Utc::now(),
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: 0.0,
            prompts: Vec::new(),
            status: AgentStatus::Idle,
            auto_commit: false,
            yolo_mode: false,
        });
        self.updated_at = Utc::now();
    }

    /// End the AI agent session
    pub fn end_agent_session(&mut self) {
        self.agent_session = None;
        self.updated_at = Utc::now();
    }
}

/// Manager for all virtual branches in a repository
#[derive(Debug, Default)]
pub struct VirtualBranchManager {
    /// All virtual branches
    branches: HashMap<String, VirtualBranch>,
    /// Path to branches storage directory
    storage_path: PathBuf,
    /// Target branch (e.g., "main" or "master")
    target_branch: String,
}

impl VirtualBranchManager {
    /// Create a new manager for a repository
    pub fn new(repo_root: &Path) -> Self {
        Self {
            branches: HashMap::new(),
            storage_path: repo_root.join(".project").join("branches"),
            target_branch: String::from("main"),
        }
    }

    /// Load all branches from storage
    pub fn load(&mut self) -> Result<()> {
        self.branches.clear();

        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path)
                .context("Failed to create branches directory")?;
            return Ok(());
        }

        for entry in fs::read_dir(&self.storage_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "json") {
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;
                let branch: VirtualBranch = serde_json::from_str(&content)
                    .with_context(|| format!("Failed to parse {}", path.display()))?;
                self.branches.insert(branch.id.clone(), branch);
            }
        }

        Ok(())
    }

    /// Save all branches to storage
    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(&self.storage_path)?;

        for branch in self.branches.values() {
            let path = self.storage_path.join(format!("{}.json", branch.id));
            let content = serde_json::to_string_pretty(branch)?;
            fs::write(&path, content)?;
        }

        Ok(())
    }

    /// Save a single branch
    pub fn save_branch(&self, branch_id: &str) -> Result<()> {
        let branch = self
            .branches
            .get(branch_id)
            .ok_or_else(|| anyhow!("Branch not found: {}", branch_id))?;

        fs::create_dir_all(&self.storage_path)?;
        let path = self.storage_path.join(format!("{}.json", branch.id));
        let content = serde_json::to_string_pretty(branch)?;
        fs::write(&path, content)?;

        Ok(())
    }

    /// Create a new virtual branch
    pub fn create_branch(&mut self, name: &str, base_commit: &str) -> Result<String> {
        // Find max order
        let max_order = self.branches.values().map(|b| b.order).max().unwrap_or(0);

        let mut branch = VirtualBranch::new(name, base_commit);
        branch.order = max_order + 1;

        let id = branch.id.clone();
        self.branches.insert(id.clone(), branch);
        self.save_branch(&id)?;

        Ok(id)
    }

    /// Delete a virtual branch
    pub fn delete_branch(&mut self, branch_id: &str) -> Result<()> {
        self.branches
            .remove(branch_id)
            .ok_or_else(|| anyhow!("Branch not found: {}", branch_id))?;

        let path = self.storage_path.join(format!("{}.json", branch_id));
        if path.exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }

    /// Get a branch by ID
    pub fn get(&self, branch_id: &str) -> Option<&VirtualBranch> {
        self.branches.get(branch_id)
    }

    /// Get a mutable branch by ID
    pub fn get_mut(&mut self, branch_id: &str) -> Option<&mut VirtualBranch> {
        self.branches.get_mut(branch_id)
    }

    /// List all branches, sorted by order
    pub fn list(&self) -> Vec<&VirtualBranch> {
        let mut branches: Vec<_> = self.branches.values().collect();
        branches.sort_by_key(|b| b.order);
        branches
    }

    /// List applied (active) branches only
    pub fn list_applied(&self) -> Vec<&VirtualBranch> {
        let mut branches: Vec<_> = self.branches.values().filter(|b| b.applied).collect();
        branches.sort_by_key(|b| b.order);
        branches
    }

    /// Find branch that owns a specific hunk
    pub fn find_hunk_owner(&self, hunk: &HunkRef) -> Option<&VirtualBranch> {
        self.branches
            .values()
            .find(|b| b.owned_hunks.contains(hunk))
    }

    /// Transfer a hunk from one branch to another
    pub fn transfer_hunk(
        &mut self,
        hunk: &HunkRef,
        from_branch_id: &str,
        to_branch_id: &str,
    ) -> Result<()> {
        // Remove from source
        let from = self
            .branches
            .get_mut(from_branch_id)
            .ok_or_else(|| anyhow!("Source branch not found"))?;
        if !from.remove_hunk(hunk) {
            return Err(anyhow!("Hunk not found in source branch"));
        }

        // Add to destination
        let to = self
            .branches
            .get_mut(to_branch_id)
            .ok_or_else(|| anyhow!("Destination branch not found"))?;
        to.assign_hunk(hunk.clone());

        // Save both
        self.save_branch(from_branch_id)?;
        self.save_branch(to_branch_id)?;

        Ok(())
    }

    /// Get/set target branch
    pub fn target_branch(&self) -> &str {
        &self.target_branch
    }

    pub fn set_target_branch(&mut self, branch: impl Into<String>) {
        self.target_branch = branch.into();
    }

    /// Get branches with active agent sessions
    pub fn active_agents(&self) -> Vec<&VirtualBranch> {
        self.branches
            .values()
            .filter(|b| {
                b.agent_session.as_ref().map_or(false, |s| {
                    !matches!(s.status, AgentStatus::Idle | AgentStatus::Completed { .. })
                })
            })
            .collect()
    }

    /// Detect all hunks currently currently in the workspace (diff vs HEAD)
    /// This is used to capture changes applied by the agent or user.
    pub fn detect_workspace_hunks(&self) -> Result<Vec<HunkRef>> {
        use git2::{DiffOptions, Repository};
        
        // Open repo
        let repo_root = self.storage_path.parent().and_then(|p| p.parent()).unwrap(); // .project/branches -> .project -> root
        let repo = Repository::open(repo_root).context("Failed to open repository")?;
        
        // Diff index to workdir (unstaged changes)
        let mut opts = DiffOptions::new();
        opts.context_lines(0); // We want minimal context for precise hashing
        opts.interhunk_lines(0);
        
        let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
        
        // We don't need the `print` call, it was just debugging.
        // And we don't need 'hunks' vec here since we delegate to collect_git2_hunks
        
        self.collect_git2_hunks(&diff)
    }

    /// Detect conflicts between virtual branches
    /// Returns a map of branch_id -> Vec of conflicting branch IDs
    pub fn detect_conflicts(&self) -> HashMap<String, Vec<String>> {
        let mut conflicts: HashMap<String, Vec<String>> = HashMap::new();
        
        let branches: Vec<_> = self.branches.values().collect();
        
        // Compare each pair of branches
        for (i, branch_a) in branches.iter().enumerate() {
            for branch_b in branches.iter().skip(i + 1) {
                // Check if any hunks overlap
                for hunk_a in &branch_a.owned_hunks {
                    for hunk_b in &branch_b.owned_hunks {
                        if hunk_a.overlaps_with(hunk_b) {
                            // Record bidirectional conflict
                            conflicts.entry(branch_a.id.clone())
                                .or_insert_with(Vec::new)
                                .push(branch_b.id.clone());
                            conflicts.entry(branch_b.id.clone())
                                .or_insert_with(Vec::new)
                                .push(branch_a.id.clone());
                        }
                    }
                }
            }
        }
        
        // Deduplicate conflict lists
        for conflicts_list in conflicts.values_mut() {
            conflicts_list.sort();
            conflicts_list.dedup();
        }
        
        conflicts
    }

    fn collect_git2_hunks(&self, diff: &git2::Diff) -> Result<Vec<HunkRef>> {
        use std::cell::RefCell;
        let hunks = RefCell::new(Vec::new());
        
        // We iterate file-by-file
        diff.foreach(
            &mut |_file, _progress| true,
            None,
            Some(&mut |delta, hunk| {
                let path = delta.new_file().path().and_then(|p| p.to_str()).unwrap_or("").to_string();
                if path.is_empty() { return true; }
                
                hunks.borrow_mut().push(HunkRef {
                    file_path: path,
                    hash: String::new(), // to be filled
                    old_start: hunk.old_start(),
                    old_count: hunk.old_lines(),
                    new_start: hunk.new_start(),
                    new_count: hunk.new_lines(),
                });
                true
            }),
            Some(&mut |_delta, _hunk, _line| {
                // We actually don't need to accumulate content here because we are deferring
                // the precise hashing logic to the other helper 'git_hunk_collector' which does it better.
                // But wait, 'detect_workspace_hunks' calls *this* function OR 'git_hunk_collector'?
                // The previous code had `detect_workspace_hunks` calling `self.collect_git2_hunks`.
                // And `collect_git2_hunks` returned `git_hunk_collector(diff)`.
                // So this whole body of `collect_git2_hunks` was redundant or seemingly unused/overwritten logic?
                
                // Ah, line 544 in the previous file was: `Ok(git_hunk_collector(diff)?)`
                // So the entire logic inside `collect_git2_hunks` UP TO that point was ignored.
                // I should just make `collect_git2_hunks` call `git_hunk_collector`.
                true
            })
        )?;
        
        // As analyzed above, the previous implementation of this function ignored the local `hunks` vec 
        // and just called `git_hunk_collector`. Let's simplify.
        Ok(git_hunk_collector(diff)?)
    }
}

/// Helper to collect hunks with content hashing
fn git_hunk_collector(diff: &git2::Diff) -> Result<Vec<HunkRef>> {
    struct BufferedHunk {
        hunk_ref: HunkRef,
        content: Vec<u8>,
    }
    
    let mut buffered_hunks: Vec<BufferedHunk> = Vec::new();
    
    // We need a RefCell because the closures capture mutable state
    use std::cell::RefCell;
    let hunks_cell = RefCell::new(&mut buffered_hunks);
    
    diff.foreach(
        &mut |_file, _progress| true,
        None,
        Some(&mut |delta, hunk| {
             let path = delta.new_file().path().and_then(|p| p.to_str()).unwrap_or("").to_string();
             
             let h = HunkRef {
                 file_path: path,
                 hash: String::new(), 
                 old_start: hunk.old_start(),
                 old_count: hunk.old_lines(),
                 new_start: hunk.new_start(),
                 new_count: hunk.new_lines(),
             };
             
             hunks_cell.borrow_mut().push(BufferedHunk {
                 hunk_ref: h,
                 content: Vec::new(),
             });
             true
        }),
        Some(&mut |_delta, _hunk, line| {
             let mut hunks = hunks_cell.borrow_mut();
             if let Some(last) = hunks.last_mut() {
                 let content = line.content();
                 last.content.extend_from_slice(content);
             }
             true
        })
    )?;
    
    // Finalize hashes
    let result = buffered_hunks.into_iter().map(|mut bh| {
        bh.hunk_ref.hash = compute_hash(std::str::from_utf8(&bh.content).unwrap_or(""));
        bh.hunk_ref
    }).collect();
    
    Ok(result)
}

/// Compute BLAKE3 hash of content (fast, SIMD-optimized)
fn compute_hash(content: &str) -> String {
    let hash = blake3::hash(content.as_bytes());
    hash.to_hex()[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_virtual_branch_creation() {
        let branch = VirtualBranch::new("feature/auth", "abc123");
        assert_eq!(branch.name, "feature/auth");
        assert_eq!(branch.base_commit, "abc123");
        assert!(branch.applied);
        assert!(branch.owned_hunks.is_empty());
    }

    #[test]
    fn test_hunk_assignment() {
        let mut branch = VirtualBranch::new("test", "abc123");
        let hunk = HunkRef::new("src/lib.rs", "fn main() {}", 1, 5, 1, 10);

        branch.assign_hunk(hunk.clone());
        assert_eq!(branch.owned_hunks.len(), 1);

        // Duplicate assignment should not add
        branch.assign_hunk(hunk.clone());
        assert_eq!(branch.owned_hunks.len(), 1);
    }

    #[test]
    fn test_hunk_staging() {
        let mut branch = VirtualBranch::new("test", "abc123");
        let hunk = HunkRef::new("src/lib.rs", "fn main() {}", 1, 5, 1, 10);

        // Cannot stage unowned hunk
        assert!(!branch.stage_hunk(&hunk));

        // Add and stage
        branch.assign_hunk(hunk.clone());
        assert!(branch.stage_hunk(&hunk));
        assert_eq!(branch.staged_hunks.len(), 1);

        // Unstage
        assert!(branch.unstage_hunk(&hunk));
        assert!(branch.staged_hunks.is_empty());
    }

    #[test]
    fn test_manager_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create and populate manager
        let mut manager = VirtualBranchManager::new(repo_root);
        let id = manager.create_branch("feature/test", "abc123").unwrap();

        // Load into new manager
        let mut manager2 = VirtualBranchManager::new(repo_root);
        manager2.load().unwrap();

        let branch = manager2.get(&id).unwrap();
        assert_eq!(branch.name, "feature/test");
    }

    #[test]
    fn test_hunk_transfer() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = VirtualBranchManager::new(temp_dir.path());

        let id1 = manager.create_branch("branch1", "abc123").unwrap();
        let id2 = manager.create_branch("branch2", "abc123").unwrap();

        let hunk = HunkRef::new("src/lib.rs", "code", 1, 5, 1, 10);

        // Assign to branch1
        manager.get_mut(&id1).unwrap().assign_hunk(hunk.clone());
        manager.save_branch(&id1).unwrap();

        // Transfer to branch2
        manager.transfer_hunk(&hunk, &id1, &id2).unwrap();

        assert!(!manager.get(&id1).unwrap().owned_hunks.contains(&hunk));
        assert!(manager.get(&id2).unwrap().owned_hunks.contains(&hunk));
    }

    #[test]
    fn test_agent_session() {
        let mut branch = VirtualBranch::new("ai-feature", "abc123");

        branch.start_agent_session("ollama:llama3");
        assert!(branch.agent_session.is_some());

        let session = branch.agent_session.as_ref().unwrap();
        assert_eq!(session.model, "ollama:llama3");
        assert_eq!(session.status, AgentStatus::Idle);

        branch.end_agent_session();
        assert!(branch.agent_session.is_none());
    }
}
