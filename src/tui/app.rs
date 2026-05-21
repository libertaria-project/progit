//! App - Application state machine
//!
//! Central state management for the TUI.

use crate::git::RepoInfo;
use crate::issue::{Issue, Status};
use crate::mr::MergeRequest;
use crate::panopticum::{PanoEvent, PanoStatus};
use crate::plugins::PluginManager;
use crate::sync::SyncProvider;
use crate::tui::style::ThemeEngine;
use crate::tui::theme::Theme;
use crate::virtual_branch::VirtualBranchManager;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

/// Current view mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    Dashboard,
    List,
    Kanban,
    Diff,
    MRList,
    Blame,
    /// Virtual branch lanes view (GitButler-style)
    Lanes,
    /// Code review mode with line-level comments
    Review,
}

/// Input mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
    Edit,
    Confirm,
    RemoteDropdown,
    BranchDropdown,
    BranchCreate,        // Typing new branch name
    BranchDeleteConfirm, // Confirm branch deletion
    VBranchCreate,       // Creating a virtual branch (typing name)
    VBranchMove,         // Selecting target lane for hunk move
    DetailView,          // Viewing issue details
    DetailEdit,          // Editing a field in detail view
    Command,             // Command palette (: command)
    MRCreate,            // Creating a merge request
    RepoFilter,          // Filtering by repository
    Settings,            // Settings pane
    FuzzyPalette,        // Fuzzy command palette (Ctrl+P)
    DiffComment,         // Adding a comment to a diff line
    ProjectWiki,         // Viewing repository-owned wiki pages
    ProjectIssues,       // Viewing repository-owned issue files
}

/// Mouse drag state
#[derive(Debug, Clone, Default)]
pub struct DragState {
    /// Issue being dragged (by ID)
    pub dragging_issue: Option<String>,
    /// Column being hovered (0=Backlog, 1=InProgress, 2=Done)
    pub hover_column: Option<usize>,
    /// Starting column
    pub start_column: Option<usize>,
}

/// Application state
pub struct App {
    /// All loaded issues
    pub issues: Vec<Issue>,

    /// Currently selected issue index
    pub selected: usize,

    /// Current view mode
    pub view_mode: ViewMode,

    /// Current input mode
    pub input_mode: InputMode,

    // Diff State
    pub diff_state: Option<crate::diff::DiffState>,

    // MR List State
    pub mr_list: Vec<crate::mr::MergeRequest>,
    pub mr_selected: usize,

    /// Search query
    pub search_query: String,

    /// Command input buffer
    pub command_input: String,

    /// Filtered issue indices
    pub filtered: Vec<usize>,

    /// Current theme
    pub theme: Theme,

    /// Current sprint number (for filtering)
    pub current_sprint: Option<u32>,

    /// Status message
    pub status_message: Option<String>,

    /// Time when status message was set (for auto-clear)
    pub status_message_time: Option<std::time::Instant>,

    /// Should quit
    pub should_quit: bool,

    /// Git repository info
    pub repo_info: Option<RepoInfo>,

    /// Mouse drag state
    pub drag_state: DragState,

    /// Selected kanban column (0=Backlog, 1=InProgress, 2=Done)
    pub kanban_column: usize,

    /// Selected index within kanban column
    pub kanban_row: usize,

    /// Selected remote in dropdown
    pub selected_remote: usize,
    /// Selected branch in dropdown
    pub selected_branch: usize,

    /// Repo filter state
    pub repo_filter: Option<String>, // None = show all, Some(repo) = filter by repo
    pub selected_repo_filter: usize, // Selected index in repo filter dropdown
    pub available_repos: Vec<String>, // Cached list of unique repos

    /// Branch pending deletion (for confirmation)
    pub pending_branch_delete: Option<String>,

    /// Issue ID being viewed in detail pane
    pub detail_issue_id: Option<String>,

    /// Which field is selected in detail view
    pub detail_field: usize,

    /// Edit buffer for text input
    pub edit_buffer: String,

    /// Last click time for double-click detection (ms since epoch)
    pub last_click_time: u128,

    /// Last clicked issue ID
    pub last_click_issue: Option<String>,

    /// Sync provider for remote synchronization
    pub sync_provider: Option<Box<dyn SyncProvider>>,

    /// Provider name (e.g. "gitlab", "forgejo")
    pub sync_provider_name: Option<String>,

    /// Sync configuration (for CI/CD plugin queries)
    pub sync_config: Option<crate::storage::config::SyncConfig>,

    /// Current sync status message
    pub sync_status: Option<String>,

    /// Draft MR being created
    pub mr_draft: Option<crate::mr::MergeRequest>,

    /// Current field in MR creation form (0=title, 1=description, 2=target_branch)
    pub mr_field: usize,

    /// Style engine
    pub theme_engine: ThemeEngine,

    /// Show debug console overlay
    pub show_debug_console: bool,

    /// Plugin manager for executing plugin hooks
    pub plugin_manager: Option<PluginManager>,

    /// Fuzzy searcher for command palette
    pub fuzzy_searcher: crate::fuzzy::FuzzySearcher,

    /// Fuzzy palette query
    pub fuzzy_query: String,

    /// Selected fuzzy result index
    pub fuzzy_selected: usize,

    /// Blame State
    pub blame_state: Option<crate::tui::widget_blame::BlameState>,

    // ─── Panopticum Integration ───────────────────────────────────────────
    /// Repository root path
    pub repo_path: PathBuf,

    /// Whether this is a Panopticum-enabled repo (PANOPTICUM.kdl exists)
    pub is_panopticum_repo: bool,

    /// Custom path to panoctl binary (None = use PATH)
    pub panoctl_binary_path: Option<String>,

    /// Current panopticum operation status
    pub pano_status: PanoStatus,

    /// Output buffer for panopticum operations (for console view)
    pub pano_output: Vec<String>,

    /// Channel sender for panopticum events (cloned to spawn functions)
    pub pano_event_tx: Option<Sender<PanoEvent>>,

    /// Channel receiver for panopticum events (polled in main loop)
    pub pano_event_rx: Option<Receiver<PanoEvent>>,

    /// Show panopticum log viewer modal
    pub show_pano_log: bool,

    // ─── Virtual Branches Integration ────────────────────────────────────────
    /// Virtual branch manager
    pub vbranch_manager: Option<VirtualBranchManager>,

    /// Selected virtual branch index in lanes view
    pub vbranch_selected: usize,

    /// Selected hunk within a virtual branch
    pub vbranch_hunk_selected: usize,

    // ─── Agent Integration ───────────────────────────────────────────────────
    /// Channel sender for agent events (cloned to agent thread)
    pub agent_event_tx: Option<Sender<crate::agent::AgentEvent>>,

    /// Channel receiver for agent events (polled in main loop)
    pub agent_event_rx: Option<Receiver<crate::agent::AgentEvent>>,

    /// Show conflict resolution modal
    pub show_conflicts: bool,

    /// Show agent menu modal
    pub show_agent_menu: bool,

    /// Selected action in agent menu
    pub agent_menu_selected: usize,

    /// Show plugin manager modal
    pub show_plugins: bool,

    /// Selected plugin index in plugin modal
    pub plugin_selected: usize,

    // ─── Code Review Integration ─────────────────────────────────────────────
    /// Review state for code review mode
    pub review_state: Option<crate::tui::widget_review::ReviewState>,

    /// Loaded repository-owned wiki view
    pub project_wiki_view: Option<crate::project_view::ProjectWikiView>,

    /// Selected page in the project wiki view
    pub project_wiki_page: usize,

    /// Vertical scroll in the project wiki page body
    pub project_wiki_scroll: u16,

    /// Loaded repository-owned issues view
    pub project_issues_view: Option<crate::project_view::ProjectIssuesView>,

    /// Selected repository-owned issue entry
    pub project_issue_selected: usize,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            selected: 0,
            view_mode: ViewMode::default(),
            input_mode: InputMode::Normal,
            search_query: String::new(),
            command_input: String::new(),
            filtered: Vec::new(),
            theme: Theme::default(),
            current_sprint: None,
            status_message: None,
            status_message_time: None,
            should_quit: false,
            repo_info: None,
            drag_state: DragState::default(),
            theme_engine: ThemeEngine::new(&std::collections::HashMap::new()), // Initialize with empty config
            kanban_column: 0,
            kanban_row: 0,

            selected_remote: 0,
            selected_branch: 0,
            repo_filter: None,
            selected_repo_filter: 0,
            available_repos: Vec::new(),
            pending_branch_delete: None,
            detail_issue_id: None,
            detail_field: 0,
            edit_buffer: String::new(),
            last_click_time: 0,
            last_click_issue: None,
            sync_provider: None,
            sync_provider_name: None,
            sync_config: None,
            sync_status: None,
            mr_draft: None,
            mr_field: 0,
            diff_state: None,
            mr_list: Vec::new(),
            mr_selected: 0,
            show_debug_console: false,
            plugin_manager: None,
            fuzzy_searcher: crate::fuzzy::FuzzySearcher::new(),
            fuzzy_query: String::new(),
            fuzzy_selected: 0,
            // Panopticum
            repo_path: PathBuf::new(),
            is_panopticum_repo: false,
            panoctl_binary_path: None,
            pano_status: PanoStatus::Idle,
            pano_output: Vec::new(),
            pano_event_tx: None,
            pano_event_rx: None,
            show_pano_log: false,
            blame_state: None,
            // Virtual Branches
            vbranch_manager: None,
            vbranch_selected: 0,
            vbranch_hunk_selected: 0,
            // Agent
            agent_event_tx: None,
            agent_event_rx: None,
            show_conflicts: false,
            show_agent_menu: false,
            agent_menu_selected: 0,
            show_plugins: false,
            plugin_selected: 0,
            review_state: None,
            project_wiki_view: None,
            project_wiki_page: 0,
            project_wiki_scroll: 0,
            project_issues_view: None,
            project_issue_selected: 0,
        }
    }

    /// Load issues into the app
    pub fn load_issues(&mut self, issues: Vec<Issue>) {
        self.issues = issues;
        self.update_available_repos(); // Update repo list for filtering
        self.fuzzy_searcher.update_issues(&self.issues); // Update fuzzy search cache
        self.refresh_filter();
    }

    /// Load MRs into the app
    pub fn load_mrs(&mut self, mrs: Vec<MergeRequest>) {
        self.mr_list = mrs;
        self.mr_selected = 0;
    }

    /// Refresh MRs from provider
    pub fn refresh_mrs(&mut self) -> anyhow::Result<()> {
        if let Some(ref provider) = self.sync_provider {
            match provider.list_mrs() {
                Ok(mrs) => {
                    self.load_mrs(mrs);
                    self.set_status(format!("Loaded {} MRs", self.mr_list.len()));

                    // Query pipeline status for all MRs
                    self.query_pipeline_status_for_all();

                    Ok(())
                }
                Err(e) => {
                    self.set_status(format!("Failed to load MRs: {}", e));
                    Err(e)
                }
            }
        } else {
            self.set_status("No sync provider configured".to_string());
            Ok(())
        }
    }

    /// Query pipeline status for all MRs via plugin
    fn query_pipeline_status_for_all(&mut self) {
        let Some(ref mut plugin_manager) = self.plugin_manager else {
            log::trace!("No plugin manager - skipping pipeline status query");
            return;
        };

        // Get forge configuration
        let (forge_type, api_url, project_id) = if let Some(ref sync_config) = self.sync_config {
            let forge = sync_config.provider.clone();
            let url = sync_config.url.clone();
            // Project ID format: owner/repo (e.g., "ProGit/progit")
            let project = format!("{}/{}", sync_config.owner, sync_config.repo);
            (forge, url, project)
        } else {
            log::trace!("No sync config - skipping pipeline status query");
            return;
        };

        log::debug!("Querying pipeline status for {} MRs via plugins", self.mr_list.len());

        // Query each MR
        for mr in self.mr_list.iter_mut() {
            let Some(remote_id) = mr.remote_id else {
                continue; // Skip MRs without remote ID
            };

            // Build event
            let event = crate::plugins::PluginEvent::PipelineStatusQuery {
                mr_id: remote_id.to_string(),
                project_id: project_id.clone(),
                source_branch: mr.source_branch.clone(),
                target_branch: mr.target_branch.clone(),
                forge_type: forge_type.clone(),
                api_url: api_url.clone(),
            };

            // Dispatch to plugins
            match plugin_manager.dispatch_event(&event) {
                Ok(responses) => {
                    // Take first valid response
                    for response in responses {
                        if let Some(status_obj) = response.as_object() {
                            if let Some(status) = status_obj.get("status").and_then(|v| v.as_str()) {
                                log::debug!("MR !{}: pipeline status = {}", remote_id, status);
                                mr.pipeline_status = Some(status.to_string());
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to query pipeline status for MR !{}: {}", remote_id, e);
                }
            }
        }
    }

    /// Refresh the filtered list based on search query
    pub fn refresh_filter(&mut self) {
        let query = if self.search_query.is_empty() {
            None
        } else {
            Some(self.search_query.to_lowercase())
        };

        self.filtered = self
            .issues
            .iter()
            .enumerate()
            .filter(|(_, i)| {
                // Search filter
                let matches_search = query.as_ref().map_or(true, |q| {
                    i.title.to_lowercase().contains(q) || i.description.to_lowercase().contains(q)
                });

                // Repo filter
                let matches_repo = self
                    .repo_filter
                    .as_ref()
                    .map_or(true, |repo| i.repo.as_ref().map_or(false, |r| r == repo));

                matches_search && matches_repo
            })
            .map(|(idx, _)| idx)
            .collect();

        // Keep selection in bounds
        if !self.filtered.is_empty() && self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    /// Update available repos list from current issues
    pub fn update_available_repos(&mut self) {
        use std::collections::HashSet;

        let mut repos: HashSet<String> = HashSet::new();
        for issue in &self.issues {
            if let Some(ref repo) = issue.repo {
                repos.insert(repo.clone());
            }
        }

        self.available_repos = repos.into_iter().collect();
        self.available_repos.sort();
    }

    /// Get currently selected issue
    pub fn selected_issue(&self) -> Option<&Issue> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.issues.get(idx))
    }

    /// Get currently selected issue (mutable)
    pub fn selected_issue_mut(&mut self) -> Option<&mut Issue> {
        self.filtered
            .get(self.selected)
            .cloned()
            .and_then(|idx| self.issues.get_mut(idx))
    }

    /// Get issue by ID (mutable)
    pub fn issue_by_id_mut(&mut self, id: &str) -> Option<&mut Issue> {
        self.issues.iter_mut().find(|i| i.id == id)
    }

    /// Move selection down
    pub fn next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    /// Move selection up
    pub fn previous(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self
                .selected
                .checked_sub(1)
                .unwrap_or(self.filtered.len() - 1);
        }
    }

    /// Toggle view mode
    pub fn toggle_view(&mut self) {
        let new_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::List,
            ViewMode::List => ViewMode::Kanban,
            ViewMode::Kanban => ViewMode::MRList,
            ViewMode::MRList => ViewMode::Dashboard,
            ViewMode::Diff => ViewMode::List,
            ViewMode::Blame => ViewMode::List,
            ViewMode::Lanes => ViewMode::List,
            ViewMode::Review => ViewMode::List,
        };

        // Auto-load MRs when switching to MR list view
        if new_mode == ViewMode::MRList && self.mr_list.is_empty() {
            let _ = self.refresh_mrs();
        }

        self.view_mode = new_mode;
    }

    /// Cycle theme
    pub fn cycle_theme(&mut self) {
        self.theme = match self.theme {
            Theme::Nord => Theme::Gruvbox,
            Theme::Gruvbox => Theme::Dracula,
            Theme::Dracula => Theme::Cyberpunk,
            Theme::Cyberpunk => Theme::Vibe,
            Theme::Vibe => Theme::Nord,
        };
    }

    /// Cycle selected issue's status
    pub fn cycle_selected_status(&mut self) {
        if let Some(issue) = self.selected_issue_mut() {
            issue.status = issue.status.next();
        }
    }

    /// Move issue to a specific status
    pub fn move_issue_to_status(&mut self, issue_id: &str, status: Status) -> bool {
        if let Some(issue) = self.issue_by_id_mut(issue_id) {
            issue.status = status;
            return true;
        }
        false
    }

    /// Set status message (expires after 3 seconds)
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_message_time = Some(std::time::Instant::now());
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
        self.status_message_time = None;
    }

    /// Get status message (auto-clears after 3 seconds)
    pub fn get_status(&mut self) -> Option<String> {
        if let Some(time) = self.status_message_time {
            if time.elapsed().as_secs() >= 3 {
                self.clear_status();
                return None;
            }
        }
        self.status_message.clone()
    }

    /// Open detail pane for an issue
    pub fn open_detail(&mut self, issue_id: &str) {
        self.detail_issue_id = Some(issue_id.to_string());
        self.detail_field = 0;
        self.edit_buffer.clear();
        self.input_mode = InputMode::DetailView;

        // Load the title into edit buffer
        if let Some(issue) = self.issues.iter().find(|i| i.id == issue_id) {
            self.edit_buffer = issue.title.clone();
        }
    }

    /// Close detail pane
    pub fn close_detail(&mut self) {
        self.detail_issue_id = None;
        self.edit_buffer.clear();
        self.input_mode = InputMode::Normal;
    }

    /// Get the issue being viewed in detail
    pub fn detail_issue(&self) -> Option<&Issue> {
        self.detail_issue_id
            .as_ref()
            .and_then(|id| self.issues.iter().find(|i| &i.id == id))
    }

    /// Get the issue being viewed in detail (mutable)
    pub fn detail_issue_mut(&mut self) -> Option<&mut Issue> {
        let id = self.detail_issue_id.clone();
        id.and_then(move |id| self.issues.iter_mut().find(|i| i.id == id))
    }

    /// Navigate to next field in detail view
    pub fn detail_next_field(&mut self) {
        self.detail_field = (self.detail_field + 1) % 9; // 9 total fields now
        self.load_field_to_buffer();
    }

    /// Navigate to previous field in detail view
    pub fn detail_prev_field(&mut self) {
        self.detail_field = if self.detail_field == 0 {
            8
        } else {
            self.detail_field - 1
        };
        self.load_field_to_buffer();
    }

    /// Load current field value into edit buffer
    pub fn load_field_to_buffer(&mut self) {
        if let Some(issue) = self.detail_issue() {
            self.edit_buffer = match self.detail_field {
                0 => issue.title.clone(),
                1 => issue.description.clone(),
                2 => issue.status.as_str().to_string(),
                3 => format!("{}", issue.effort as u8),
                4 => issue.assignee.clone().unwrap_or_default(),
                5 => issue.tags.join(", "),
                6 => issue
                    .due
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
                7 => issue
                    .started
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
                8 => issue
                    .completed
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
                _ => String::new(),
            };
        }
    }

    /// Save edit buffer to current field
    pub fn save_field_from_buffer(&mut self) {
        let buffer = self.edit_buffer.clone();
        let field = self.detail_field;
        if let Some(issue) = self.detail_issue_mut() {
            match field {
                0 => issue.title = buffer,
                1 => issue.description = buffer,
                // Status and Effort handled by cycling
                4 => {
                    issue.assignee = if buffer.is_empty() {
                        None
                    } else {
                        Some(buffer)
                    }
                }
                5 => {
                    issue.tags = buffer
                        .split(|c| c == ',' || c == ';')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                6 => {
                    // Start dragging out the date parsing to a helper if reuse is needed,
                    // but for now, let's just support multiple formats inline to keep it simple and contiguous.
                    // Due Date (end of day)
                    if buffer.is_empty() {
                        issue.due = None;
                    } else if let Some(dt) = parse_date_input(&buffer) {
                        issue.due = Some(chrono::DateTime::from_naive_utc_and_offset(
                            dt.and_hms_opt(23, 59, 59).unwrap(),
                            chrono::Utc,
                        ));
                    }
                }
                7 => {
                    // Started Date (start of day)
                    if buffer.is_empty() {
                        issue.started = None;
                    } else if let Some(dt) = parse_date_input(&buffer) {
                        issue.started = Some(chrono::DateTime::from_naive_utc_and_offset(
                            dt.and_hms_opt(0, 0, 0).unwrap(),
                            chrono::Utc,
                        ));
                    }
                }
                8 => {
                    // Completed Date (end of day/now)
                    if buffer.is_empty() {
                        issue.completed = None;
                    } else if let Some(dt) = parse_date_input(&buffer) {
                        issue.completed = Some(chrono::DateTime::from_naive_utc_and_offset(
                            dt.and_hms_opt(23, 59, 59).unwrap(),
                            chrono::Utc,
                        ));
                    }
                }
                _ => {}
            }
            issue.updated = chrono::Utc::now();
        }
    }

    /// Group issues by status (for kanban view)
    pub fn issues_by_status(&self) -> (Vec<&Issue>, Vec<&Issue>, Vec<&Issue>) {
        let backlog: Vec<_> = self
            .issues
            .iter()
            .filter(|i| i.status == Status::Backlog)
            .collect();
        let in_progress: Vec<_> = self
            .issues
            .iter()
            .filter(|i| i.status == Status::InProgress)
            .collect();
        let done: Vec<_> = self
            .issues
            .iter()
            .filter(|i| i.status == Status::Done)
            .collect();
        (backlog, in_progress, done)
    }

    /// Get issues for a specific column
    pub fn issues_for_column(&self, column: usize) -> Vec<&Issue> {
        let status = match column {
            0 => Status::Backlog,
            1 => Status::InProgress,
            _ => Status::Done,
        };
        self.issues.iter().filter(|i| i.status == status).collect()
    }

    /// Navigate kanban: move right
    pub fn kanban_right(&mut self) {
        self.kanban_column = (self.kanban_column + 1).min(2);
        self.clamp_kanban_row();
    }

    /// Navigate kanban: move left
    pub fn kanban_left(&mut self) {
        self.kanban_column = self.kanban_column.saturating_sub(1);
        self.clamp_kanban_row();
    }

    /// Navigate kanban: move down
    pub fn kanban_down(&mut self) {
        let col_issues = self.issues_for_column(self.kanban_column);
        if !col_issues.is_empty() {
            self.kanban_row = (self.kanban_row + 1) % col_issues.len();
        }
    }

    /// Navigate kanban: move up
    pub fn kanban_up(&mut self) {
        let col_issues = self.issues_for_column(self.kanban_column);
        if !col_issues.is_empty() {
            self.kanban_row = self
                .kanban_row
                .checked_sub(1)
                .unwrap_or(col_issues.len() - 1);
        }
    }

    /// Clamp kanban row to valid range
    fn clamp_kanban_row(&mut self) {
        let col_issues = self.issues_for_column(self.kanban_column);
        if col_issues.is_empty() {
            self.kanban_row = 0;
        } else if self.kanban_row >= col_issues.len() {
            self.kanban_row = col_issues.len() - 1;
        }
    }

    /// Get selected issue in kanban view
    pub fn kanban_selected_issue(&self) -> Option<&Issue> {
        let col_issues = self.issues_for_column(self.kanban_column);
        col_issues.get(self.kanban_row).copied()
    }

    /// Move selected kanban issue to next column
    pub fn kanban_move_right(&mut self) -> bool {
        if self.kanban_column >= 2 {
            return false;
        }
        if let Some(issue) = self.kanban_selected_issue() {
            let id = issue.id.clone();
            let new_status = match self.kanban_column {
                0 => Status::InProgress,
                1 => Status::Done,
                _ => return false,
            };
            return self.move_issue_to_status(&id, new_status);
        }
        false
    }

    /// Move selected kanban issue to previous column
    pub fn kanban_move_left(&mut self) -> bool {
        if self.kanban_column == 0 {
            return false;
        }
        if let Some(issue) = self.kanban_selected_issue() {
            let id = issue.id.clone();
            let new_status = match self.kanban_column {
                1 => Status::Backlog,
                2 => Status::InProgress,
                _ => return false,
            };
            return self.move_issue_to_status(&id, new_status);
        }
        false
    }

    /// Calculate velocity (total done points)
    pub fn velocity(&self) -> u32 {
        self.issues
            .iter()
            .filter(|i| i.status == Status::Done)
            .map(|i| i.effort as u32)
            .sum()
    }

    /// Count blockers
    pub fn blocker_count(&self) -> usize {
        self.issues.iter().filter(|i| i.is_blocker()).count()
    }

    /// Load blame for a file
    pub fn load_blame(&mut self, file_path: &str) {
        self.set_status(format!("Loading blame for {}...", file_path));

        // blame_file expects path relative to repo root
        match crate::git::blame::BlameInfo::new(file_path) {
            Ok(info) => {
                let mut state = crate::tui::widget_blame::BlameState::default();
                state.info = Some(info);
                self.blame_state = Some(state);
                self.view_mode = ViewMode::Blame;
                self.set_status(format!("Blame: {}", file_path));
            }
            Err(e) => {
                self.set_status(format!("Failed to load blame: {}", e));
            }
        }
    }

    /// Open repository-owned wiki pages in a read-only TUI overlay.
    pub fn open_project_wiki(&mut self) {
        match crate::project_view::load_project_wiki(&self.repo_path) {
            Ok(view) => {
                let count = view.pages.len();
                self.project_wiki_view = Some(view);
                self.project_wiki_page = 0;
                self.project_wiki_scroll = 0;
                self.input_mode = InputMode::ProjectWiki;
                self.set_status(format!("Loaded {} project wiki page(s)", count));
            }
            Err(err) => {
                self.set_status(format!("Project wiki unavailable: {}", err));
            }
        }
    }

    /// Open repository-owned issue files in a read-only TUI overlay.
    pub fn open_project_issues(&mut self) {
        match crate::project_view::load_project_issues(&self.repo_path) {
            Ok(view) => {
                let count = view.issues.len();
                self.project_issues_view = Some(view);
                self.project_issue_selected = 0;
                self.input_mode = InputMode::ProjectIssues;
                self.set_status(format!("Loaded {} project issue file(s)", count));
            }
            Err(err) => {
                self.set_status(format!("Project issues unavailable: {}", err));
            }
        }
    }

    /// Close repository-owned project overlays.
    pub fn close_project_overlay(&mut self) {
        self.input_mode = InputMode::Normal;
        self.project_wiki_scroll = 0;
    }
}

/// Helper: Parse date from string (supports YYYY-MM-DD and YYYYMMDD)
fn parse_date_input(input: &str) -> Option<chrono::NaiveDate> {
    // Try ISO format (YYYY-MM-DD)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Some(date);
    }
    // Try compact format (YYYYMMDD)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(input, "%Y%m%d") {
        return Some(date);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Effort;
    use std::fs;

    #[test]
    fn test_app_navigation() {
        let mut app = App::new();
        app.load_issues(vec![
            Issue::new("Issue 1"),
            Issue::new("Issue 2"),
            Issue::new("Issue 3"),
        ]);

        assert_eq!(app.selected, 0);

        app.next();
        assert_eq!(app.selected, 1);

        app.next();
        assert_eq!(app.selected, 2);

        app.next(); // Wrap around
        assert_eq!(app.selected, 0);

        app.previous(); // Wrap back
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_search_filter() {
        let mut app = App::new();
        app.load_issues(vec![
            Issue::new("Fix authentication"),
            Issue::new("Add dashboard"),
            Issue::new("Auth refactor"),
        ]);

        app.search_query = "auth".to_string();
        app.refresh_filter();

        assert_eq!(app.filtered.len(), 2);
    }

    #[test]
    fn test_velocity() {
        let mut app = App::new();
        app.load_issues(vec![
            Issue::new("Done 1")
                .with_status(Status::Done)
                .with_effort(Effort::Large), // 5
            Issue::new("Done 2")
                .with_status(Status::Done)
                .with_effort(Effort::Small), // 2
            Issue::new("In Progress").with_status(Status::InProgress),
        ]);

        assert_eq!(app.velocity(), 13);
    }

    #[test]
    fn test_kanban_navigation() {
        let mut app = App::new();
        app.load_issues(vec![
            Issue::new("Backlog 1"),
            Issue::new("InProgress 1").with_status(Status::InProgress),
        ]);

        assert_eq!(app.kanban_column, 0);
        app.kanban_right();
        assert_eq!(app.kanban_column, 1);
        app.kanban_right();
        assert_eq!(app.kanban_column, 2);
        app.kanban_right(); // Should stay at 2
        assert_eq!(app.kanban_column, 2);
    }

    #[test]
    fn test_open_project_wiki_loads_overlay() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".project/wiki")).unwrap();
        fs::write(dir.path().join(".project/wiki/index.md"), "# Index\n").unwrap();
        fs::write(
            dir.path().join(".project/wiki/manifest.kdl"),
            r#"
wiki {
    version 0
    root ".project/wiki/index.md"

    page "index" {
        title "Index"
        path ".project/wiki/index.md"
        required true
    }
}
"#,
        )
        .unwrap();

        let mut app = App::new();
        app.repo_path = dir.path().to_path_buf();
        app.open_project_wiki();

        assert_eq!(app.input_mode, InputMode::ProjectWiki);
        assert_eq!(app.project_wiki_view.as_ref().unwrap().pages.len(), 1);
    }

    #[test]
    fn test_open_project_issues_loads_overlay() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".project/issues")).unwrap();
        let issue = Issue::new("Project issue");
        fs::write(
            dir.path().join(".project/issues/issue.json"),
            serde_json::to_string(&issue).unwrap(),
        )
        .unwrap();

        let mut app = App::new();
        app.repo_path = dir.path().to_path_buf();
        app.open_project_issues();

        assert_eq!(app.input_mode, InputMode::ProjectIssues);
        assert_eq!(app.project_issues_view.as_ref().unwrap().issues.len(), 1);
    }
}
