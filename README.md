# ProGit

**A blazingly fast, AI-powered Git workflow manager with virtual branches**

[![License: LCL-1.0](https://img.shields.io/badge/License-LCL--1.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.6.1--beta-green.svg)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

> **Status:** Beta - Phase 2 (Collaboration) Complete | Phase 3 (Ecosystem) In Progress

## 🎯 What is ProGit?

ProGit is a **terminal-based Git workflow manager** that combines the power of:

- **Virtual Branches** (GitButler-style) - Work on multiple features simultaneously
- **Code Review** - Line-level commenting with inline indicators
- **CI/CD Integration** - Real-time pipeline status from GitLab/GitHub
- **AI Agents** - Refactor code, generate tests, find bugs, write docs
- **Conflict Resolution** - Visual conflict detection and resolution
- **Local-First** - Your data lives in your repo, not the cloud
- **Lightning Fast** - 10MB binary, <100ms cold start

**The Pitch:** _"GitButler's virtual branches + GitHub Copilot's AI + GitUI's speed = ProGit"_

---

## ✨ Key Features

### 🌿 Virtual Branches

Work on multiple changes simultaneously without traditional Git branching chaos:

- **Parallel Workflows**: Edit different features side-by-side
- **Hunk-Level Control**: Assign code changes to different virtual branches
- **Visual Lanes**: See all your work streams at once
- **Conflict Detection**: Real-time warnings when hunks overlap
- **Easy Staging**: Drag-and-drop hunks between branches

```
┌──────────────┬──────────────┬──────────────┐
│ Feature A    │ Bug Fix B    │ Refactor C   │
├──────────────┼──────────────┼──────────────┤
│ + add_user() │ - fix null   │ ~ rename var │
│ + tests      │ + validation │ ~ extract fn │
└──────────────┴──────────────┴──────────────┘
```

### 🤖 AI-Powered Workflow

Built-in AI agent with 7 curated actions:

| Action | Description |
|--------|-------------|
| 📖 **Explain Changes** | Code review assistant |
| 🧪 **Generate Tests** | Unit test generator |
| ♻️ **Refactor Code** | Structure improvements |
| 📝 **Add Documentation** | Docstring generator |
| 🐛 **Find Bugs** | Static analysis |
| ⚡ **Optimize Performance** | Algorithm optimizer |
| 💬 **Generate Commit Msg** | Conventional Commits |

**Usage:**
1. Press `a` in Lanes view
2. Select an AI action
3. Agent analyzes your code and applies changes

### 📝 Code Review Mode (Phase 2 ✨)

Line-level code review system with inline comments:

- **Enter Review**: `:review <file> [commit]`
- **Navigate**: j/k keys to move between lines
- **Comment**: Press `c` to add a comment on any line
- **Inline Indicators**: 💬 emoji shows commented lines
- **Persistent**: Comments stored in `.project/reviews/`
- **Sidebar View**: See author, date, and comment text

```
┌────────────────────────────────┬────────────────────────┐
│ 42 💬 fn calculate_total() {   │ Comments               │
│ 43    let sum = 0;             │                        │
│ 44    for item in items {      │ alice • 2025-01-30     │
│ 45        sum += item.price;   │ Consider using iter()  │
│ 46    }                         │ .sum() here for better │
│ 47    sum                       │ readability.           │
└────────────────────────────────┴────────────────────────┘
```

### 🔄 CI/CD Integration (Phase 2 ✨)

Real-time pipeline status in Merge Request Dashboard:

- **Status Icons**: ✓ passed, ✗ failed, ● running, ○ pending
- **Color-Coded**: Green/red/yellow indicators
- **Plugin-Based**: Uses GitLab CI plugin (extensible)
- **No Browser**: Check CI status without leaving terminal

| MR | State | CI | Title | Author |
|----|-------|----|-------|--------|
| !42 | Open | ✓ | Add auth | alice |
| !41 | Open | ✗ | Fix bug | bob |
| !40 | Open | ● | Refactor | charlie |

**Configuration:**
```kdl
plugins {
    gitlab-ci {
        gitlab_api_url "https://gitlab.com/api/v4"
        gitlab_token "glpat-xxxxxxxxxxxxxxxxxxxx"
    }
}
```

### 🔍 Conflict Resolution

Visual conflict detection and resolution:

- **Real-time Detection**: ⚠️ indicators when hunks overlap
- **Side-by-side View**: Compare conflicting changes
- **Smart Merging**: Resolve conflicts with keyboard navigation
- **Prevention First**: See conflicts before they become problems

### 🎨 Beautiful TUI

- **Themes**: Cyberpunk, Nord, Dracula, Solarized, custom themes
- **Vim Keybindings**: j/k navigation, modal editing
- **Fuzzy Palette** (Ctrl+P): Jump to any issue/file/commit
- **Status Bar**: Context-aware help text

---

## 🚀 Quick Start

### Installation

```bash
# From source
git clone https://git.maiwald.work/ProGit/progit
cd progit
cargo build --release

# Binary will be at target/release/prog
sudo cp target/release/prog /usr/local/bin/
```

### First Run

```bash
# Navigate to a Git repo
cd your-project/

# Initialize ProGit
prog init

# Start the TUI
prog
```

### Basic Workflow

```bash
# Open ProGit
prog

# Create virtual branches (press 'n' in Lanes view)
# Make code changes in your editor
# Assign hunks to branches (press 'h'/'l' to switch lanes)
# Stage hunks (press 'Space')
# Commit branch (press 'C')

# Use AI agent (press 'a')
# Select action → Agent analyzes code → Auto-apply changes
```

---

## 📖 Documentation

- [**Virtual Branches Guide**](docs/VIRTUAL_BRANCHES.md) - Complete guide to virtual branches
- [**Plugin SDK**](docs/PLUGIN_SDK.md) - Write Lua plugins (Apache 2.0)
- [**Contributing**](CONTRIBUTING.md) - How to contribute
- [**Changelog**](CHANGELOG.md) - Version history
- [**Roadmap**](ROADMAP.md) - Planned features

---

## 🏗️ Architecture

ProGit is built with a **clean separation of concerns**:

```
┌─────────────────────────────────────┐
│  TUI (LCL-1.0)                      │  ← You interact here
│  - Virtual branches                 │
│  - Conflict resolution              │
│  - Agent menu                       │
└─────────────────────────────────────┘
           ↓ JSON Events
┌─────────────────────────────────────┐
│  Plugin SDK (LSL-1.0)               │  ← Write plugins here
│  - Event system                     │
│  - LuaJIT runtime                   │
└─────────────────────────────────────┘
           ↓ File System
┌─────────────────────────────────────┐
│  Data Layer                         │
│  - Issues: .project/issues/*.json   │
│  - Config: .project/config.kdl      │
│  - State: .progit/ (gitignored)     │
└─────────────────────────────────────┘
```

### License Architecture

| Component | License | Why |
|-----------|---------|-----|
| **Core TUI** | LCL-1.0 | File-level copyleft, modifications stay open |
| **Plugin SDK** | LSL-1.0 | File-level copyleft + patent grant, commercial-friendly |
| **Your Data** | Yours | JSON in your repo, you own it |

---

## 🔌 Plugin System

Extend ProGit with Lua plugins:

```lua
-- hello.lua
local plugin = {
    metadata = {
        name = "hello",
        version = "1.0.0"
    }
}

function plugin:on_event(event)
    if event.type == "IssueCreated" then
        print("New issue: " .. event.data.issue_id)
    end
end

return plugin
```

**Community Plugins** (coming soon):
- Jira Sync
- Slack Notifications
- CI/CD Integration
- Custom Git Hooks

See [Plugin SDK Documentation](docs/PLUGIN_SDK.md) for details.

---

## 🎯 Roadmap

### v0.6.0-beta (Current)
- [x] Virtual branches
- [x] Conflict detection
- [x] AI agent menu
- [x] Plugin SDK (LuaJIT runtime)
- [x] Code review mode
- [x] CI/CD integration (GitLab/Forgejo)

### v1.0.0 (Q2 2026)
- [ ] Web UI (separate product)
- [ ] Cloud sync (optional)
- [ ] Mobile companion app
- [ ] Enterprise features

See [ROADMAP.md](ROADMAP.md) for full details.

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Development setup
- Code style guide
- Testing requirements
- PR workflow

**Quick Links:**
- [Good First Issues](https://git.maiwald.work/ProGit/progit/issues)
- [Discord Community](https://discord.gg/progit)
- [Discussions](https://git.maiwald.work/ProGit/progit/issues)

---

## 📊 Comparison

### vs GitButler

| Feature | ProGit | GitButler |
|---------|--------|-----------|
| Binary Size | **~10MB** | ~200MB |
| Cold Start | **<100ms** | ~2s |
| AI Integration | ✅ Built-in | ❌ No |
| Local-First | ✅ Yes | ✅ Yes |
| Plugin System | ✅ Lua/WASM | ❌ No |
| License | LCL-1.0 (file-level copyleft) | Proprietary |

### vs GitHub CLI

| Feature | ProGit | GitHub CLI |
|---------|--------|------------|
| Virtual Branches | ✅ Yes | ❌ No |
| TUI | ✅ Full | ⚠️ Limited |
| AI Agents | ✅ 7 actions | ❌ No |
| Offline Work | ✅ Full | ⚠️ Limited |
| Issue Management | ✅ Built-in | ✅ Via API |

### vs GitUI/lazygit

| Feature | ProGit | GitUI | lazygit |
|---------|--------|-------|---------|
| Virtual Branches | ✅ Yes | ❌ No | ❌ No |
| AI Assistance | ✅ Yes | ❌ No | ❌ No |
| Plugin System | ✅ Yes | ❌ No | ❌ No |
| Speed | ⚡ Fast | ⚡ Fast | ⚡ Fast |
| Issue Tracking | ✅ Built-in | ❌ No | ❌ No |

---

## 🙏 Credits

ProGit stands on the shoulders of giants:

- **GitButler** - Virtual branches inspiration
- **GitHub Copilot** - AI-assisted coding vision
- **GitUI/lazygit** - TUI excellence
- **Ratatui** - Terminal UI framework
- **LuaJIT** - Lightning-fast plugin runtime

---

## 📜 License

**ProGit Core:** LCL-1.0 (Libertaria Commonwealth License)
**Plugin SDK:** LSL-1.0 (Libertaria Sovereign License)

Both licenses use **file-level copyleft** (modifications to our files stay open, your larger works stay yours). LSL-1.0 includes explicit patent grant for corporate use.

See [LICENSE](LICENSE) for full text.

---

## 🌟 Star History

If you find ProGit useful, please star the repo! ⭐

---

**Made with ❤️ by the ProGit Team**

[Website](https://progit.dev) • [Documentation](https://docs.progit.dev) • [Source](https://git.maiwald.work/ProGit/progit) • [Discord](https://discord.gg/progit)
