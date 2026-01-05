# 🚀 ProGit

> **"Your Code. Your Repository. Your Rules. Your Issues."**  
> *The Terminal-First Forge Alternative.*

## **We're Rivaling GitLab, GitHub, and Forgejo.**

### **With a Single Binary. And a Plugin Ecosystem.**

**Your issues belong in YOUR repository. Not their database.**

For too long, GitHub and GitLab have held your project's issues hostage. They sell "collaboration" while locking your data behind web UIs, API rate limits, and vendor lock-in.

**ProGit breaks the chains.**

We're not just an issue tracker. We're building a **complete forge alternative** that runs in your terminal:

- ✅ **Issue Tracking** (Done - you're using it now)
- ✅ **Merge Request Creation** (Done - press `M`)
- ✅ **Multi-Repo Management** (Done - manage frontend/backend/infra in ONE TUI)
- 🚧 **Code Review in Terminal** (Coming - comment on diffs without browser)
- 🚧 **CI/CD Pipeline Viewer** (Coming - watch builds in real-time)
- 🚧 **Wiki/Docs Management** (Coming - markdown files, git-tracked)
- 🚧 **[Plugin System](docs/SDK.md)** (Coming - extend with Lua/WASM)

Store your issues as **JSON files** directly in your git repository. Track them, diff them, review them in pull requests. Your issues are YOUR data, versioned alongside your code, accessible forever—even if GitHub shuts down tomorrow.

## 🔥 **What Makes ProGit Different?**

### **Multi-Repository Management** (GitLab/GitHub Can't Do This!)

Manage issues across **multiple related repositories** in a single TUI:

```
📋 Issues (13 total) │ 📦 frontend: 5 │ backend: 3 │ infra: 2

ID       Title              Status      Effort  Repo       Tags
abc123   Fix auth bug       in-progress 5       frontend   security
def456   API optimization   backlog     3       backend    performance
ghi789   Deploy pipeline    done        8       infra      devops
```

**One ProGit instance. Multiple repos. Color-coded. Filterable. Synced correctly.**

Perfect for:
- **Monorepo teams** (track all services in one view)
- **Microservices** (frontend, backend, infra issues together)
- **Multi-project coordination** (see the big picture)

Configure once, manage forever:
```kdl
repos {
    repo "frontend" {
        sync { provider "gitlab" url "https://gitlab.com" ... }
    }
    repo "backend" {
        sync { provider "forgejo" url "https://git.example.com" ... }
    }
}
```

### **What We Deliver:**

  - 🎯 **Full Kanban Board** (drag-and-drop, visual status)
  - 📊 **Time Tracking** (due dates, effort estimates, velocity)
  - 🔀 **Browser-Free MR Creation** (press `M`, done)
  - 🔍 **Git Blame View** (dual-perspective authorship attribution)
  - 🌓 **Staged/Unstaged Diffs** (side-by-side view with `Tab` toggle)
  - 📐 **Hunk Folding** (collapse/expand diff hunks with `h`)
  - 📦 **Multi-Repo Management** (one TUI, infinite repos)
  - 🔄 **Bidirectional Sync** (GitLab, Forgejo, GitHub coming soon)
  - ⚡ **Blazing Fast TUI** (Pure Rust, 5MB binary, no Electron bloat)
  - 🎭 **Beautiful Themes** (Nord, Gruvbox, Dracula, Cyberpunk, Vibe)
  - 💾 **Git-Native Storage** (`.project/issues/*.json` — transparent, diffable files)
  - 🔒 **100% Local-First** (work offline, sync when YOU decide)
  - ⌨️ **Keyboard-Driven** (Vim-style navigation, command palette)
  - 🆓 **Zero Subscriptions** (EUPL-1.2 license, free forever)

**This is the forge, democratized.**

> *"What GitLab sells, we give away. What GitHub hides, we expose. Your code, your issues, your repository. One binary. Infinite sovereignty."*

📊 **[See how ProGit compares to Jira, Linear, and GitHub Issues →](COMPARISON.md)**

-----

## 🎬 Screenshots

### Kanban View
![Kanban View](Kanban-View.webp)
*Drag-and-drop cards, visual glow for blockers/active tasks*

### List View
![List View](List-View.webp)
*Table view with search, sorting, and color-coded rows*

### Detail View
![Detail View](Detail-View.webp)
*Full issue editing with due dates, assignees, and tags*

-----

## 🚀 Quick Start

**Trust code, not blobs.** Build it yourself in seconds.

```bash
# Clone & Build
git clone https://git.maiwald.work/SSSS/progit
cd progit
cargo build --release

# Run
./target/release/prog

# Or install globally
cargo install --path .
```

### First Launch

ProGit auto-initializes in any Git repository:

```bash
cd your-project/
prog  # Creates .project/ (tracked) and .progit/ (ignored) automatically
```

-----

## ⌨️ Keyboard Shortcuts

### Global

  - `Tab` - Toggle List ↔ Kanban
  - `t` - Cycle Themes (Nord → Gruvbox → Dracula → Cyberpunk)
  - `n` - New Issue
  - `S` - Sync (Push/Pull)
  - `/` - Search
  - `q` - Quit

### Navigation (Vim-style)

  - `hjkl` - Navigate (←↓↑→)
  - `Enter` - Open Details
  - `Space` - Cycle Status (Backlog → In Progress → Done)

### Kanban

  - `H/L` - Move Card Left/Right
  - `Mouse` - Drag & Drop supported

### Detail View

  - `Tab` / `Shift+Tab` - Navigate Fields
  - `Enter` - Edit Field
  - `Space` - Cycle Status/Effort
  - `Esc` - Close

### Blame View

  - `Ctrl+P` then `b` - Open Blame for selected file
  - `j/k` - Scroll through blame lines
  - `m` - Toggle Manager/Lead Dev mode
  - `q` - Close Blame view

### Diff View

  - `j/k` - Scroll diff lines
  - `J/K` - Previous/Next file in diff
  - `Space` - Toggle file collapsing
  - `h` - Toggle hunk folding
  - `c` - Add comment to selected line
  - `Tab` - Toggle Staged/Unstaged mode
  - `q` / `Esc` - Close Diff view

-----

## 📦 CLI Commands

```bash
# Sync
prog sync push   # Push local changes to remote
prog sync pull   # Pull remote changes

# Management
prog due <id> 2025-12-31    # Set deadline
prog due <id> clear         # Remove deadline
prog block <id>             # Toggle 'Blocked' status
prog clean                  # Prune duplicates

# Branches & MRs
prog branch list            # List branches
prog branch create <name>   # Create and switch to branch
prog mr list                # List open MRs
prog mr create              # Create MR from current branch
```

-----

## 🔧 Configuration

Config lives in `.project/config.kdl`:

```kdl
sync {
    provider "gitlab"
    url "https://gitlab.com"
    owner "myteam"
    repo "myproject"
}
// Theme preference is persisted automatically
```

-----

## 🎨 Features

### Visual Status System ("Kanban Glow")

  - 🔴 **Red** - Blocked or Overdue (Critical)
  - 🟢 **Green** - In Progress (Active)
  - ⚫ **Gray** - Done (Archived)
  - **Default** - Backlog (Idle)

### Time Tracking

  - **Due Date** - Hard deadlines.
  - **Started/Completed** - Auto-timestamping on status change.
  - **Overdue Detection** - Auto-highlights late issues in Red.

### Smart Sync

  - **Timestamp Strategy** - Newer change wins (Local vs Remote).
  - **Username Mapping** - Auto-resolves Usernames to IDs.
  - **Bidirectional** - One command keeps the forge and the CLI in sync.

### Status Bar Intelligence

  - Default: `📊 2/5 done │ 1 active │ 🔥 2 blocked`
  - Live velocity tracking.
  - Temporary alerts expire after 3s.

-----

## 🏗️ Architecture

**Maiwald's PANOPTICUM** - One feature, one index, one folder.

```
src/
├── issue/          # Domain Model
│   ├── model.rs    # Core Structs
│   └── operations.rs
├── storage/        # JSON Persistence
│   ├── kdl.rs      # Config Parsing
│   └── json.rs     # Issue Storage (Source of Truth)
├── sync/           # Forge Adapters
│   ├── gitlab.rs
│   └── forgejo.rs
└── tui/            # Interface
    ├── app.rs      # State Machine
    └── theme.rs    # Visuals
```

### Storage Philosophy

  - **JSON** (`.project/issues/*.json`) - The Source of Truth. Optimized for Web App Sync & CRUD.
  - **KDL** (`.project/config.kdl`) - Configuration Only. Human-editable.
  - **Git-Backed** - Every issue is a file. Every change is a commit.

-----

## 🛣️ Roadmap: The Terminal Forge

> **[📅 View 2025 Strategic Roadmap](docs/plans/2025_roadmap.md)**

### **Phase 1: Issue Sovereignty** ✅ DONE
  - [x] Kanban Board with drag-and-drop
  - [x] List view with search/filter
  - [x] Detail editing (all fields)
  - [x] GitLab/Forgejo bidirectional sync
  - [x] Browser-free MR creation (press `M`)
  - [x] Beautiful themes (Vibe, Nord, Gruvbox, etc.)

### **Phase 2: Code Review Liberation** 🚧 IN PROGRESS
  - [x] Git Blame View (Dual-perspective authorship)
  - [x] Staged/Unstaged Diff Viewer (with hunk folding)
  - [x] MR list view (see all open MRs)
  - [x] Comment on code lines (TUI interface ready)
  - [ ] Approve/reject MRs from terminal
  - [ ] Multi-provider sync (GitLab + Forgejo simultaneously)

### **Phase 3: CI/CD Visibility** 🔜 NEXT
  - [ ] Pipeline status in status bar
  - [ ] Live build logs in TUI
  - [ ] Job retry/cancel from terminal
  - [ ] Artifact browser

### **Phase 4: Complete Forge** 🎯 VISION
  - [ ] Wiki/Docs management (markdown, git-tracked)
  - [ ] Release management
  - [ ] Container registry browser
  - [ ] Plugin system (Lua/WASM)
  - [ ] Custom workflows

### **The Goal:**
**Replace GitLab/GitHub/Forgejo with a single 5MB binary.**

No web UI. No Electron. No vendor lock-in. Just pure terminal power.

-----

## 📜 Philosophy

> **"Management by Exception: Green means go. Red means stop."**

ProGit follows **sane & proven** principles:

  - **Local-First:** Your data, your machine.
  - **Minimal Ceremony:** No story mapping, SAFe, or planning poker.
  - **Visual Signals:** Colors process faster than text.
  - **Keyboard-Driven:** Speed over decoration.
  - **Sync When Ready:** Not internet-dependent.

No vendor lock-in. No SaaS fees. No tracking.

-----

## 🤝 Contributing

Built with ❤️ in **Rust**.

```bash
git clone https://git.maiwald.work/SSSS/progit
cargo test
cargo fmt
```

## ⚖️ Licensing: The Libertaria Model

This project is governed by the **Libertaria License Suite**, enforcing total reciprocity for the core while enabling a flourishing ecosystem.

| Component | License | Identifier | Principle |
|-----------|---------|------------|-----------|
| **ProGit Core** | Commonwealth License | **[LCL-1.0](LICENSE)** | **Strong Copyleft.** Service loophole closed. If you run it, you share it. |
| **Plugin SDK** | Unbound License | **[LUL-1.0](LICENSE_SDK)** | **Permissive.** Build closed or open plugins. Zero friction adoption. |
| **Enterprise** | Venture License | **[LVL-1.0](LICENSE_VENTURE)** | **Glass Box.** Closed source allowed, but build provenance is mandatory. |

> *"Code for the common good, or not at all."*

-----

**Made with 🔥 by developers, for developers.**

*Stop clicking. Start shipping.*
