# Changelog

All notable changes to ProGit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0-beta] - 2025-01-30

### 🎉 Phase 2: Collaboration Layer - COMPLETE

**Strategic Achievement:** All Phase 2 features implemented. Binary size: 9.9MB (under 10MB target).

#### 📝 Code Review Mode

Complete line-level code review system:

- **`:review <file> [commit]`** - Enter review mode for any file
- **Line-level commenting** - Press `c` to add comments on any line
- **Inline indicators** - 💬 emoji marks commented lines
- **Comments sidebar** - Shows author, date, and text
- **Persistent storage** - Comments saved to `.project/reviews/`
- **Visual diff view** - Syntax-colored diff with navigation (j/k)
- **Keyboard-driven** - No mouse required for entire workflow

**Implementation:**
- `src/review.rs` - Storage system (Review, ReviewComment, ReviewStorage)
- `src/tui/widget_review.rs` - UI rendering with split view
- Full integration with TUI input system

#### 🔄 CI/CD Pipeline Integration

Real-time CI/CD status in Merge Request Dashboard:

- **Pipeline status column** - Shows status for each MR
- **Color-coded indicators** - ✓ (passed), ✗ (failed), ● (running), ○ (pending)
- **Plugin-based architecture** - Validates entire plugin system
- **Event-based API** - New `PipelineStatusQuery` event type
- **GitLab CI plugin** - First production plugin (Lua-based)
- **No hardcoded logic** - All CI queries through plugin system

**Strategic Win:** CI/CD is a plugin, not core code. This validates the plugin architecture and prevents binary bloat.

**Implementation:**
- `plugins/gitlab-ci/main.lua` - GitLab API integration
- `src/plugins/manager.rs` - Added `dispatch_event()` method
- `src/plugins/sdk.rs` - Added `PipelineStatusQuery` event
- `src/mr/model.rs` - Added `pipeline_status` field
- `src/tui/widget_mr_list.rs` - Added CI column rendering
- `src/tui/app.rs` - Added `query_pipeline_status_for_all()`

#### 🔌 Plugin SDK Enhancement

Extended Plugin trait with event-based API:

- **`on_event()` method** - Query-style plugin interactions
- **Bidirectional communication** - Plugins can return response data
- **Backward compatible** - Existing hook-based plugins unaffected
- **JSON-only boundary** - Maintains trait firewall

**progit-plugin-sdk changes:**
- `src/traits/core.rs` - Added `on_event()` to Plugin trait
- `src/lua/mod.rs` - Added `call_event()` implementation
- Enables request-response patterns alongside lifecycle hooks

#### 🔀 Merge Request Dashboard

Enhanced MR list view:

- **CI/CD status column** - Real-time pipeline status
- **Color-coded states** - Visual state indicators
- **Keyboard navigation** - j/k to navigate MRs
- **Query on demand** - Fetches status when viewing dashboard

#### 📋 MR Model Updates

- Added `pipeline_status: Option<String>` field
- Updated all MR constructors (forgejo, gitlab, local)
- Stores status: "passed", "failed", "running", "pending", etc.

### 🎯 Phase 2 Completion Metrics

| Feature | Status | Progress |
|---------|--------|----------|
| MR Dashboard | ✅ Done | 100% |
| CI/CD Status Display | ✅ Done | 100% |
| CI/CD Plugin System | ✅ Done | 100% |
| Code Review Mode | ✅ Done | 100% |
| Conflict Resolution | ✅ Done | 100% |
| Virtual Branches | ✅ Bonus | 100% |

**Overall Phase 2: 100%** 🎉

### 🏗️ Architectural Validation

- ✅ Plugin system works end-to-end
- ✅ Trait firewall enforced (no Lua types in core)
- ✅ JSON-only communication boundary
- ✅ Binary size preserved at 9.9MB
- ✅ Event-based API superior to pure hooks
- ✅ Data sovereignty maintained (.project/ storage)

### 📚 Documentation

- **PHASE2_COMPLETE.md** - Comprehensive Phase 2 summary
- **PHASE2_STATUS.md** - Status tracking and metrics
- **plugins/gitlab-ci/README.md** - Plugin documentation
- **progit-market/plugins/gitlab-ci.json** - Marketplace entry

### 🚀 What's Next: Phase 3 (Ecosystem Layer)

Next priorities:
1. Plugin Registry - `prog plugin install <name>`
2. Binary Diet - Remove syntect → syntax-highlight plugin → <8MB
3. Marketplace UI - Search/browse plugins in TUI
4. Plugin Dependencies - Version management

### Technical Details

- **22 files changed** in core repo
- **2 files changed** in plugin SDK
- **1,678 insertions** total
- **0 compile errors** ✅
- Plugin: gitlab-ci (Lua, 165 lines)
- Review system: 308 lines (storage + UI)

## [0.5.2-beta] - 2026-01-16

### 🎯 Phase 1: "Addiction Molecules"

#### 🪝 Git Hooks Integration
- Smart commit-msg hook auto-updates issue status
- `prog hooks install/uninstall/status` CLI commands
- Keywords: `closes/fixes/resolves #ID` → marks Done
- Keywords: `refs/see/re #ID` → marks In Progress
- Preserves existing user hooks when installing
- Issue IDs support alphanumeric format (e.g., `#abc-123`)

#### 📝 Markdown Live Renderer
- Issue descriptions render as styled markdown in detail view
- Supports: headers, **bold**, *italic*, `code`, lists, > quotes
- Uses `pulldown-cmark` parser (~105 KiB)
- Only renders when not in edit mode (seamless editing)

#### ⚡ Binary Size Optimization
- **17 MiB → 11 MiB** (-35% reduction)
- Switched from vendored OpenSSL to system OpenSSL
- Added comprehensive size audit: `docs/BINARY_SIZE_AUDIT.md`
- Identified future optimization paths (feature flags)

### 🔌 Plugin System (Continued)

#### Plugin CLI Integration
- `prog plugin list` - Show installed plugins with version info
- `prog plugin install <path>` - Install .lua plugins locally
- `prog plugin remove <name>` - Uninstall plugins
- `prog plugin info <name>` - Show plugin metadata and hooks
- Auto-load plugins from `plugins/` and `.progit/plugins/`
- Plugins trigger on issue create/delete events

### 🌿 Virtual Branches (Interactive)

#### Keyboard Operations Wired
- **n** - Create new virtual branch (prompts for name)
- **Space/Enter** - Toggle hunk staging in selected lane
- **m** - Move hunk to different lane (interactive target selection)
- **c** - Conflict resolution modal for overlapping hunks

### Technical
- Added `regex` crate for issue reference parsing
- Added `pulldown-cmark` 0.13 for markdown rendering
- Created `src/hooks.rs` module (287 lines)
- Created `src/tui/markdown.rs` module (244 lines)
- Plugin manager now supports directory-based loading
- All 75 tests passing

## [0.4.0-alpha] - 2026-01-14

### Major Features

#### 🤖 AI Agent Menu System
- Interactive modal menu with 7 curated AI actions
- Action-specific prompts for specialized workflows:
  - 📖 Explain Changes - Code review assistant
  - 🧪 Generate Tests - Unit test generator
  - ♻️ Refactor Code - Structure improvements  
  - 📝 Add Documentation - Docstring generator
  - 🐛 Find Bugs - Static analysis
  - ⚡ Optimize Performance - Algorithm optimizer
  - 💬 Generate Commit Message - Conventional Commits
- Beautiful centered modal (60% width, 70% height)
- j/k navigation, Enter to execute, Esc to cancel
- Context-aware execution (reads files from virtual branch hunks)
- Non-blocking background agent execution

#### 🔌 Plugin SDK Foundation
- Apache 2.0 licensed plugin SDK for community extensions
- `Plugin` trait - implementation agnostic API
- `PluginEngine` trait - abstracts LuaJIT/WASM runtimes
- 9 plugin event types (IssueCreated, CommitCreated, etc.)
- Plugin registry for managing installed plugins
- Complete API documentation in `docs/PLUGIN_SDK.md`
- Example auto-tagger plugin in `examples/plugins/`

### Improvements
- Agent actions now use custom prompts per task type
- System prompts configure agent persona dynamically
- Status bar shows agent execution progress
- Modal overlays render with proper z-index prioritization

### Technical
- Created `src/tui/agent_executor.rs` for action execution
- Created `src/tui/widget_agent_menu.rs` for menu UI
- Created `src/plugins/sdk.rs` with Apache 2.0 license
- Added mlua 0.11 dependency for LuaJIT support (~2MB)
- Plugin SDK separates from core TUI (license firewall)

## [0.3.0-alpha] - 2026-01-14

### Major Features

#### 🌿 Virtual Branches (Production Ready)
- GitButler-style virtual branches for parallel workflows
- Hunk-level assignment to virtual branches
- Visual lanes in TUI (h/l to navigate)
- Real-time conflict detection between branches
- ⚠️ warning indicators for conflicting hunks
- Drag-and-drop hunks between lanes (m key)
- Stage and commit individual branches

#### 🔀 Conflict Resolution UI
- Beautiful conflict resolution modal (press 'c')
- Side-by-side conflict visualization
- List conflicting branches with details
- Shows overlapping hunks with file paths and line ranges
- Actionable guidance for resolution
- Escape key to close and return to workflow

#### 🤖 AI Agent Integration
- Initial agent integration with Ollama
- Context gathering from virtual branch hunks
- Background thread execution (non-blocking)
- Agent applies generated diffs automatically
- System prompts configure agent behavior

### Improvements
- Conflict detection runs O(n²) branch comparison (optimized for typical loads)
- Modal rendering system for overlays
- Event channel for agent communication (mpsc)
- Blake3 hashing for efficient hunk identification

### Technical
- Created `src/virtual_branch.rs` with full branch management
- Created `src/tui/widget_lanes.rs` for visual lanes
- Created `src/tui/widget_conflicts.rs` for conflict UI
- Created `src/agent/context.rs` for code context gathering
- Added conflict detection algorithm with overlap checking

## [0.2.0-alpha] - 2025-12-11

### Major Features

#### 📋 Issue Management
- Local JSON-based issue storage
- Kanban board view
- Sprint planning
- Issue status workflow
- Markdown rendering in issue views

#### 🎨 Theme System
- Multiple built-in themes (Cyberpunk, Nord, Dracula, Solarized)
- Custom theme support via KDL config
- Live theme switching
- Consistent color palette system

#### 🔍 Fuzzy Command Palette
- Global fuzzy search (Ctrl+P)
- Jump to issues, files, commits in <200ms
- Sublime Text-style navigation
- Keyboard-first workflow

### Improvements
- Vim-style keybindings (j/k navigation)
- Modal input system (Normal/Command/Edit modes)
- Status bar with context-aware help
- Settings panel for configuration

### Technical
- Ratatui-based TUI framework
- KDL configuration format
- Crossterm for terminal handling
- Git2 for repository operations

## [0.1.0-alpha] - 2025-11-15

### Initial Release

#### Core Features
- Terminal UI with Ratatui
- Git repository detection
- Basic issue listing
- Configuration system

#### Storage
- JSON-based issue storage
- KDL configuration format
- Local-first data model

#### Infrastructure
- EUPL-1.2 license for core
- Rust 1.75+ required
- Linux/BSD primary targets

---

## Version Numbering

**Alpha (0.x.x):** Feature development, breaking changes expected  
**Beta (0.5.x):** Feature freeze, bug fixes only  
**Stable (1.x.x):** Production ready, semantic versioning

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to propose changes.

## License

ProGit Core: **EUPL-1.2** (copyleft)  
Plugin SDK: **Apache-2.0** (permissive, allows commercial use)

[unreleased]: https://github.com/yourusername/progit/compare/v0.4.0-alpha...HEAD
[0.4.0-alpha]: https://github.com/yourusername/progit/compare/v0.3.0-alpha...v0.4.0-alpha
[0.3.0-alpha]: https://github.com/yourusername/progit/compare/v0.2.0-alpha...v0.3.0-alpha
[0.2.0-alpha]: https://github.com/yourusername/progit/compare/v0.1.0-alpha...v0.2.0-alpha
[0.1.0-alpha]: https://github.com/yourusername/progit/releases/tag/v0.1.0-alpha
