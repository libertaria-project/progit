# ProGit: GitButler's Virtual Branches + AI + Plugins in a 5MB Binary

I've been frustrated with Git workflow tools for a while. GitButler has amazing virtual branches, but it's ~200MB and proprietary. GitHub Copilot has AI, but no virtual branches. lazygit is fast, but no AI or advanced features.

So I built **ProGit** — a terminal-based Git workflow manager that combines all three.

## What is it?

ProGit lets you work on multiple features simultaneously without traditional Git branching chaos:

- **Virtual Branches** (GitButler-style): Assign code hunks to different feature branches side-by-side
- **AI Agents**: 7 built-in actions (explain code, generate tests, refactor, find bugs, optimize performance)
- **Conflict Detection**: Real-time warnings when hunks overlap
- **Plugin System**: LuaJIT runtime for community extensions
- **Local-First**: Issues stored as JSON in your repo, not vendor databases

**Binary size:** 5MB (vs GitButler's ~200MB)  
**Cold start:** <100ms  
**License:** LCL-1.0 (core) + LSL-1.0 (plugins)

## Why I built this

As a SysOps/DevOps person, I needed:

1. **Multi-feature workflow**: Work on bug fix + feature + refactor simultaneously
2. **AI assistance**: Refactor code without leaving terminal
3. **Speed**: No waiting for Electron apps to start
4. **Data sovereignty**: Issues belong in my repo, not GitHub's database
5. **Extensibility**: Lua plugins for custom integrations (Jira sync, Slack notifications, etc.)

## Demo

```
# Open ProGit in any Git repo
cd your-project/
prog

# Create virtual branches (press 'n' in Lanes view)
# Make code changes in your editor
# Assign hunks to branches (press 'h'/'l' to switch)

# Use AI agent (press 'a')
Select action:
  📖 Explain Changes
  🧪 Generate Tests
  ♻️  Refactor Code
  🐛 Find Bugs
  ⚡ Optimize Performance

# Agent analyzes your code and auto-applies changes
```

## Technical Highlights

**Virtual Branches:**
- Hunk-level assignment to branches
- Visual lanes (navigate with h/l)
- Blake3 hashing for efficient tracking
- Conflict detection (O(n²) branch comparison)

**AI Integration:**
- Ollama client (local LLMs)
- Context gathering from virtual branch hunks
- Action-specific prompts (each AI action has custom system prompt)
- Non-blocking execution (background threads)

**Plugin System:**
- LSL-1.0 licensed SDK (allows commercial plugins)
- LuaJIT runtime (~2MB binary impact)
- Event system (IssueCreated, CommitCreated, etc.)
- Example plugin: auto-tagger (keywords → tags)

**Architecture:**
```
TUI (LCL-1.0) → Plugin SDK (LSL-1.0) → Data (JSON in repo)
```

This license firewall lets the core stay copyleft while allowing proprietary plugins.

## Comparisons

**vs GitButler:**
- 5MB vs ~200MB binary
- <100ms vs ~2s cold start
- ✅ AI built-in (GitButler: no)
- ✅ Plugin system (GitButler: no)
- LCL-1.0 vs Proprietary

**vs GitHub CLI:**
- ✅ Virtual branches (gh: no)
- ✅ Full TUI (gh: limited)
- ✅ AI agents (gh: no)
- ✅ Offline-first (gh: API-dependent)

**vs lazygit/GitUI:**
- ✅ Virtual branches (neither has this)
- ✅ AI assistance (neither has this)
- ✅ Plugin system (neither has this)
- Similar speed (all are fast)

## Current Status

**Alpha complete** (v0.4.0):
- ✅ Virtual branches with conflict detection
- ✅ AI agent menu (7 actions)
- ✅ Plugin SDK + LuaJIT runtime
- ✅ Professional documentation

**Next (v0.5.0-beta):**
- Finish thread-safe plugin loading
- Example plugins (Jira sync, commit linter, Slack notifications)
- Plugin marketplace

## Try it

```bash
git clone https://github.com/yourusername/progit
cd progit
cargo build --release
./target/release/prog
```

Feedback welcome! Especially from:
- Developers who juggle multiple features
- People frustrated with vendor lock-in
- Anyone who wants AI in their terminal workflow

GitHub: [github.com/yourusername/progit](https://github.com/yourusername/progit)

---

**Why "ProGit"?**  
Professional Git. Programmable Git. Progressive Git. Take your pick.

**Why LCL-1.0?**  
Strong copyleft (like AGPL) but EU-friendly and compatible with GPL.

**Why LuaJIT for plugins?**  
2MB runtime, instant startup, Neovim developers love it. WASM is Phase 2 for banks who demand sandboxing.

**How's it compare to Magit (Emacs)?**  
Similar power-user focus, but:
- ✅ Virtual branches (Magit: traditional Git)
- ✅ AI agents (Magit: no)
- ✅ Standalone binary (Magit: requires Emacs)

**Monetization plan?**  
- Core: Free forever (LCL-1.0)
- Plugins: LSL-1.0 (commercial OK)
- Future: Cloud sync SaaS, Enterprise support

---

Curious what HN thinks. Is this solving a real problem or am I just scratching my own itch?
