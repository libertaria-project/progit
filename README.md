# 🚀 ProGit

> **"Your Code. Your Repository. Your Rules. Your Issues."**  
> *Decentralized Issue Tracking for the Ungovernable.*

## **We Unfuck GitHub & GitLab.**

**Your issues belong in YOUR repository. Not their database.**

For too long, GitHub and GitLab have held your project's issues hostage. They sell "collaboration" while locking your data behind web UIs, API rate limits, and vendor lock-in.

**ProGit breaks the chains.**

Store your issues as **human-readable KDL files** directly in your git repository. Track them, diff them, review them in pull requests. Your issues are YOUR data, versioned alongside your code, accessible forever—even if GitHub shuts down tomorrow.

We deliver what they gatekeep:

  - 🎯 **Full Kanban Board** (drag-and-drop, visual status)
  - 📊 **Time Tracking** (due dates, effort estimates, velocity)
  - 🔄 **Bidirectional Sync** (GitLab, Forgejo, GitHub coming soon)
  - ⚡ **Blazing Fast TUI** (Pure Rust, 5MB binary, no Electron bloat)
  - 🎭 **Beautiful Themes** (Nord, Gruvbox, Dracula, Cyberpunk)
  - 💾 **Git-Native Storage** (`.project/issues/*.kdl` — transparent, diffable files)
  - 🔒 **100% Local-First** (work offline, sync when YOU decide)
  - ⌨️ **Keyboard-Driven** (Vim-style navigation, command palette)
  - 🆓 **Zero Subscriptions** (EUPL-1.2 license, free forever)

**This is issue tracking, democratized.**

> *"What GitLab sells, we give away. What GitHub hides, we expose. Your code, your issues, your repository."*

📊 **[See how ProGit compares to Jira, Linear, and GitHub Issues →](https://www.google.com/search?q=COMPARISON.md)**

-----

## 🎬 Screenshots

### Kanban View

*Drag-and-drop cards, visual glow for blockers/active tasks*

### List View

*Table view with search, sorting, and color-coded rows*

### Detail View

*Full issue editing with due dates, assignees, and tags*

-----

## 🚀 Quick Start

**Trust code, not blobs.** Build it yourself in seconds.

```bash
# Clone & Build
git clone https://github.com/yourusername/progit
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
├── storage/        # Dual KDL/JSON Persistence
│   ├── kdl.rs      # Human-readable (Tracked)
│   └── json.rs     # Machine-fast Cache (Ignored)
├── sync/           # Forge Adapters
│   ├── gitlab.rs
│   └── forgejo.rs
└── tui/            # Interface
    ├── app.rs      # State Machine
    └── theme.rs    # Visuals
```

### Storage Philosophy

  - **KDL** (`.project/issues/*.kdl`) - The Source of Truth. Merge-conflict friendly.
  - **JSON** (`.progit/cache.json`) - The Speed Layer. Rebuilt on launch.
  - **Dual Sync** - Edit the KDL manually? The JSON updates. Edit in TUI? The KDL updates.

-----

## 🛣️ Roadmap

  - [ ] GitHub Sync Adapter
  - [ ] Bulk Operations (Tagging, Status)
  - [ ] Command Palette (`:`)
  - [ ] CSV/JSON Export
  - [ ] Custom Fields
  - [ ] Attachments Support

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
git clone https://github.com/yourusername/progit
cargo test
cargo fmt
```

**License:** [EUPL-1.2](LICENCE) (Free as in Freedom).

-----

**Made with 🔥 by developers, for developers.**

*Stop clicking. Start shipping.*

-----

### 🗡️ The Blade's Edits:

1.  **Intro:** Tightened "We give you everything they charge for" to "We deliver what they gatekeep" for more punch.
2.  **Quick Start:** Added **"Trust code, not blobs"** to implicitly address the binary/security fear.
3.  **Storage Philosophy:** Clarified *why* the dual system exists (Source of Truth vs. Speed Layer) to appeal to the engineers.
4.  **Formatting:** Standardized capitalization on feature lists for better scannability.