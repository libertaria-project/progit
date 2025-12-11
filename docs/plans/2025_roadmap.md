# ProGit Roadmap 2025

**Mission**: Build a terminal-based development environment that outperforms web interfaces through local-first speed and keyboard-driven efficiency.

## Core Features (Completed)

- **Local-First Storage**: Atomic, JSON-based storage engine synced with Git tracked files.
- **TUI Engine**: Dynamic layouts, theming engine, and Vim-style navigation.
- **Project Management**: Full issue tracking, Kanban boards, and sprint planning.
- **Fuzzy Command Palette**: Global fuzzy search for issues, commands, and files (`Ctrl+P`).

## In Development

### Interactive Rebase Visualizer
A graphical interface for complex git rebase operations, replacing the text editor todo list with an interactive TUI component.

### Advanced Diff Viewer
Syntax-highlighted side-by-side diffs within the terminal for rapid code review.

## Plugin & Extension Architecture

We are building a robust, business-friendly plugin system to allow endless customization and corporate integration without bloating the core binary.

- **ProGit Plugin SDK**: A standalone Rust Crate for developing extensions.
- **Dual Runtime Support**:
    - **LuaJIT**: Lightweight, fast runtime for scripting and automation (Default).
    - **WASM**: Secure, sandboxed runtime for complex integrations (Optional Feature).
- **Enterprise Integration**: Capabilities for proprietary logic, identity management, and custom workflows via isolated plugin hooks.

## Future Capabilities

- **Merge Request Review**: Context-aware code review interface.
- **CI/CD Intelligence**: Real-time pipeline status and log streaming.
- **Release Management**: Semantic versioning and changelog automation.
- **Snippet Management**: Local and shared code snippets.

---

> "The scalpel that makes the Surgeon faster than the Administrator."
