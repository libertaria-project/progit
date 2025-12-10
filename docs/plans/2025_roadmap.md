# ProGit Strategic Roadmap: Operation Workflow Superiority

**Objective**: Build a terminal-based development environment that is significantly faster than the GitLab Web UI.
**Timeline**: 6 Months (4 Sprints)

## 🏁 Sprint 1: The Foundation (Month 1-2)
**Goal**: Unbreakable local storage and lightning-fast issue management.
**Status**: Near Completion.

- [x] **Storage Engine**: JSON-based (Optimized for Web App Sync & CRUD), atomic, local-first.
- [x] **TUI Core**: Interactivity, settings, themes, dynamic layouts.
- [x] **Project Management**: Issues, Kanban, Sprints (Dogfooding).
- [ ] **Reliability**: Unit test coverage for storage & sync logic.

## 🚀 Sprint 2: The Killer Feature - Code Review (Month 3-4)
**Goal**: Make the terminal the superior place to review code. "I don't open the browser to approve a PR."

- [ ] **MR Dashboard**: specialized view for Merge Requests (Author, Reviewers, Pipeline Status).
- [ ] **Diff Viewer**: syntax-highlighted, side-by-side diffs in TUI.
- [ ] **Review Comments**: add/reply to comments directly on code lines.
- [ ] **Checkout MR**: One-key context switch to the contributor's branch.
- [ ] **Approvals**: `A` to Approve, `R` to Request Changes.

## ⚡ Sprint 3: The Ecosystem - CI/CD (Month 5)
**Goal**: Immediate feedback loop. "Why did it fail?" answered in milliseconds.

- [ ] **Pipeline Status**: Live indicators in status bar.
- [ ] **Log Viewer**: Stream build logs in real-time with ANSI colors.
- [ ] **Retry/Cancel**: Control jobs without leaving the context.
- [ ] **Local Verify**: Run pre-commit hooks / local checks via TUI before pushing.

## 🛠️ Sprint 4: The Forge - Completeness (Month 6)
**Goal**: A self-contained forge environment.

- [ ] **Wiki/Docs**: Manage git-backed documentation.
- [ ] **Release Management**: Tag versions, generate changelogs.
- [ ] **Plugin System**: Lua/WASM extensions.
- [ ] **Addon Packs**: Community themes and icons ("Pimp My ProGit").
- [ ] **Security**: GPG signing and identity integration.

---

> "We won't build GitLab. We will build the scalpel that makes the Surgeon faster than the Administrator."
