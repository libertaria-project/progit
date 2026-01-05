# 🎄 ProGit v0.2.0 - Christmas Release

**Release Date:** December 28, 2025  
**Tag:** v0.2.0  
**Binary Size:** 16MB (stripped, optimized)  

---

## 🎁 Major Features

### Side-by-Side Diff Engine
The most requested feature is here! View code changes in a beautiful side-by-side layout with full syntax highlighting, directly in your terminal.

**Features:**
- **Split Pane Layout**: OLD code on the left, NEW code on the right
- **Syntax Highlighting**: Powered by `syntect` with base16-ocean.dark theme
- **Smart Alignment**: Intelligent line-by-line comparison algorithm
- **Mouse Support**: Click files in the left pane to switch between them
- **Keyboard Navigation**:
  - `j/k` - Scroll through diff
  - `J/K` - Switch between files
  - `space` - Collapse/expand files
  - `q` - Exit diff view

**Triggers:**
- Press `d` anywhere to see uncommitted local changes (working tree vs index)
- Press `Enter` on an MR to review what will be merged (target...source)
- Use `:diff [ref]` command to compare against any Git reference

---

## 🔒 Keybinding Safety Improvements

We've swapped the diff and delete keybindings for better UX:

| Key | Action | Reason |
|-----|--------|--------|
| **`d`** | Quick Diff | Easy access for frequent action |
| **`D`** | Delete Issue | Requires Shift (safer) |

This change is applied consistently across:
- Dashboard view
- List view
- Kanban view
- Branch dropdown

---

## 📚 SDK Documentation

Published comprehensive SDK architecture documentation:

**docs/SDK.md** includes:
- Why the SDK exists (Extensibility Paradox)
- Architecture layers (Core, SDK, Plugins)
- Plugin lifecycle and hooks
- WASM roadmap for Sprint 4

**Enterprise Idea Backlog:**
- CI/CD Token Auth integration
- GitLab Knowledge Graph (GKG) integration for impact analysis
- MCP (Model Context Protocol) support for AI agents

---

## 🛠️ Technical Improvements

### Diff Engine Internals
- **Git2 Integration**: Native Git diff operations via `libgit2`
- **Alignment Algorithm**: `align_lines()` for side-by-side rendering
- **RefCell Pattern**: Borrow-checker-safe Git callbacks
- **Mouse Click Handling**: `UIAreas::diff_file_list` for precise file selection
- **Syntax Caching**: `once_cell` lazy static for efficient syntax loading

### Code Quality
- Fixed borrow checker errors in diff.rs
- Corrected `DiffState::load()` signature across all call sites
- Added `FileDiff::collapsed` field for UI state
- Static helper functions to avoid self-borrowing issues

---

## 📦 New Files

- `docs/SDK.md` - Complete SDK architecture and roadmap
- `src/tui/widget_dashboard.rs` - Redesigned dashboard with keybindings
- `src/diff.rs` - Completely refactored diff engine
- `.project/mrs.json` - MR persistence

---

## 🐛 Bug Fixes

- Fixed syntax errors in help text displays
- Corrected dashboard keybinding display
- Updated context-aware help for all views
- Fixed file list click detection edge cases

---

## 🎯 UX Enhancements

- **Dashboard "Quick Start" Guide**: Visual keybinding reference with `[d] Quick Diff`
- **Status Bar Help**: Context-aware help shows `d:diff` in List/Kanban modes
- **File List Indicators**: Collapse state shown with ▶/▼ icons
- **Smooth Scrolling**: Automatic scroll reset when switching files
- **Syntax-Aware Colors**: Language-specific highlighting for better readability

---

## 🚀 Installation

### From Source
```bash
git clone https://git.maiwald.work/SSSS/progit
cd progit
git checkout v0.2.0
cargo build --release
./target/release/prog
```

### Binary
Download the 16MB optimized binary from the releases page.

---

## 🎄 Contributors

- **Voxis Forge** (AI Development Partner) - Diff Engine implementation
- **Markus Maiwald** (Lead Developer) - Architecture & Integration

---

## 📊 Statistics

- **Lines Changed**: +4,052 / -967 (main branch)
- **Files Modified**: 40
- **New Features**: 8
- **Bug Fixes**: 7
- **Warnings**: 63 (non-blocking)
- **Build Time**: 1m 08s (release)
- **Binary Size**: 16MB (stripped)

---

## 🎅 Message from the Team

This Christmas release represents months of careful design and implementation, bringing GitLab/GitHub-level code review capabilities directly to your terminal.

No bloat. No Electron. No vendor lock-in.

**Just pure, blazing-fast TUI excellence.**

Your code. Your repository. Your rules. Your Diff Engine.

**Merry Christmas from ProGit! 🎄**

---

## 🔗 Links

- **Repository**: https://git.maiwald.work/SSSS/progit
- **Tag**: v0.2.0
- **License**: EUPL-1.2
- **Documentation**: docs/SDK.md

