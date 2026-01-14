# Changelog

All notable changes to ProGit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- LuaJIT plugin runtime integration (in progress)
- Plugin command execution via TUI
- Plugin registry management

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
