# ProGit v0.3.0 "Weihnachtsmann Alpha II" - Release Notes

**"The Feature Enhancement Release"**

This release marks a significant milestone in the visual and functional capabilities of ProGit. While we remain in Alpha, this version transforms the application from a simple issue tracker into a robust, keyboard-driven forge alternative.

## 🌟 Highlights

### ⚡ Global Keybindings & Navigation
- **Universal Shortcuts**: Search (`/`), Command Palette (`:`), and Fuzzy Find (`Ctrl+P`) now work from ANY view.
- **Unified Navigation**: Consistent `vim`-style navigation across Dashboard, Kanban, List, and Diff views.
- **Context Awareness**: Specific actions (like `c` for comment) are context-aware, while global actions remain accessible.

### 🎨 Visual & Interactive Improvements
- **Diff View Comments**: You can now append comments directly to lines in the diff view (press `c`).
- **Merge Request List**: New dedicated view to list and manage Merge Requests across multiple providers.
- **UI Polish**: Smoother transitions and consistent status bar updates.

### ⚖️ New Legal Framework
- **Libertaria License Mobile**: We have adopted the **LCL-1.0 (Commonwealth)** license for the core binary.
- **Plugin Ecosystem**: Introduced the **LUL-1.0 (Unbound)** license for the SDK to encourage broad adoption.
- **Enterprise Ready**: Defined the **LVL-1.0 (Venture)** license for verifying commercial builds.

## 🛠 Fixes & Refactors
- **Cleaned Up Codebase**: Removed unused imports, variables, and deprecated Ratatui method calls.
- **Enhanced Error Handling**: Better feedback in the status bar for git operations.
- **Performance**: Optimized key handling dispatch logic.

## 🔜 What's Next?
- **Stability Focus**: The `main` branch will now focus on hardening existing features.
- **SDK Development**: Work on the Plugin SDK moves to the `develop` branch.
- **CI/CD Integration**: Viewing pipeline status directly in the TUI.

---

*"Your Code. Your Repository. Your Rules."*
