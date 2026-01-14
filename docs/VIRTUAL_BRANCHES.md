# 🌲 Virtual Branches: The New Source of Truth

**Work on multiple features simultaneously without switching git branches.**

> "Git branches are heavyweight. Virtual branches are light. They are just metadata over your working directory."

---

## The Problem

In traditional Git, you can only have **one** active branch checked out at a time. If you are working on Feature A and need to fix a bug in Feature B, you must:
1. `git stash` your changes (or make a WIP commit)
2. `git checkout feature-B`
3. Fix the bug
4. `git checkout feature-A`
5. `git stash pop`

This context switching is expensive and error-prone. It's even worse when you have 3-4 features in flight.

## The Solution: Virtual Branches

ProGit implements a **Virtual Branching** model inspired by GitButler.

- **One Working Directory**: You stay on your main branch (e.g., `main` or `develop`).
- **Multiple Virtual Lanes**: You create virtual branches that "own" specific hunks of your changes.
- **Selective Committing**: When you commit, you only commit the hunks belonging to that virtual branch.

### How It Works

1. **Unified Diff**: All your local changes are visible in a unified diff view.
2. **Hunk Ownership**: You assign specific code changes (hunks) to specific virtual branches.
3. **Independent Staging**: Each virtual branch has its own staging area.
4. **Agent Integration**: AI Agents work on specific virtual branches, modifying code in isolation (conceptually) while sharing the same file system.

---

## 🎨 The Interface (Lanes View)

Press `V` (Shift+v) to enter the **Lanes View**.

```
┌─────────────┬─────────────┬─────────────┐
│  feat/auth  │  fix/login  │  Unassigned │
│  🤖 working │  ⚠️ conflict│             │
├─────────────┼─────────────┼─────────────┤
│ [src/auth.rs]             │ [src/main.rs]
│ + fn login  │             │ + let x = 1 │
├─────────────┼─────────────┤             │
│ Staged (1)  │ Staged (0)  │             │
│ [auth.rs]   │             │             │
└─────────────┴─────────────┴─────────────┘
```

### Keybindings

| Key | Action |
|-----|--------|
| `h` / `l` | Move selection between lanes (branches) |
| `j` / `k` | Move selection within a lane (hunks) |
| `n` | Create a new virtual branch |
| `Space` | Stage/Unstage the selected hunk |
| `m` | Move selected hunk to another lane |
| `Enter` | Commit the staged hunks for this branch |

---

## 💾 Data Model

Virtual branches are stored as JSON metadata in `.project/branches/`. They are **local-only** until you push them to the remote.

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "feature/user-auth",
  "base_commit": "abc1234...",
  "owned_hunks": [
    {
      "file_path": "src/auth.rs",
      "content_hash": "a1b2c3d4...",
      "lines": [10, 25]
    }
  ],
  "staged_hunks": [],
  "agent_session": null
}
```

This means your virtual branch definitions can be checked into git (if you want to share them) or gitignored (for personal workflow).

---

## 🤖 AI Agent Integration

Each virtual branch can have an **AI Agent** assigned to it.
- The agent effectively "owns" the branch.
- It can read the hunks assigned to it.
- It can propose new changes (which appear as new unowned hunks or auto-assigned hunks).
- You act as the **Orchestrator**, reviewing the agent's work in its lane before committing.

---

## ⚠️ Conflicts

Since all branches share the same working directory, conflicts can occur if two branches modify the same lines.
- ProGit detects these conflicts immediately.
- Conflicting branches are marked with `⚠️`.
- You must resolve the hunk ownership (decide which branch "wins" or if they can coexist) before committing.
