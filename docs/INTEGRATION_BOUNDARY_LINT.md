# Integration Boundary Lint

**Status:** Living document — updated as commands migrate to plugins  
**Purpose:** Track which repo-level commands touch outside systems, and where they belong.

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Clean — no external touch |
| ⚠️ | External touch — migration planned |
| 🔌 | External touch — already plugin architecture |
| 🏗️ | In transition (shim extracted, not yet standalone) |

---

## Core Commands (must stay in core)

| Command | External Touch? | Notes |
|---------|-----------------|-------|
| `prog init` | ✅ | Creates `.project/` only |
| `prog issue list` | ✅ | Reads local JSON/KDL |
| `prog issue new` | ✅ | Writes local JSON/KDL |
| `prog issue show <id>` | ✅ | Reads local JSON/KDL |
| `prog issue status <id>` | ✅ | Writes local JSON/KDL |
| `prog issue block` | ✅ | Writes local JSON/KDL |
| `prog mr list` | ✅ | Reads local JSON/KDL |
| `prog mr show <id>` | ✅ | Reads local JSON/KDL |
| `prog mr branch <id>` | ✅ | Local git branch only |
| `prog project validate` | ✅ | Validates `.project/` layout |
| `prog project wiki` | ✅ | Reads `.project/wiki/` |
| `prog project issues` | ✅ | Reads `.project/issues/` |
| `prog diff` | ✅ | Runs `git diff` (git data plane) |
| `prog rebase` | ✅ | Runs `git rebase -i` (git data plane) |
| `prog blame` | ✅ | Runs `git blame` (git data plane) |
| `prog review` | ✅ | Local review state only |

## Integration Commands (must migrate to plugins)

| Command | External Touch? | Current Location | Future Home | Status |
|---------|-----------------|------------------|-------------|--------|
| `prog sync push` | ⚠️ | `src/sync/` | `sync-gitlab`, `sync-forgejo` plugins | Not started |
| `prog sync pull` | ⚠️ | `src/sync/` | `sync-gitlab`, `sync-forgejo` plugins | Not started |
| `prog remote doctor` | ⚠️ | `src/remote.rs` | `sync-*` plugin family | Not started |
| `prog review push` | ⚠️ | `src/main.rs:451` | `sync-*` plugin family | Not started |
| `prog citadel` | 🏗️ | `src/citadel/` | Standalone `citadel` plugin (Phase 4) | **Shim extracted** |
| `prog sober` | 🔌 | `src/sober.rs` | Marketplace plugin `sober-raccoon` | **Already plugin** |

## Plugin Commands (correctly external)

| Command | External Touch? | Notes |
|---------|-----------------|-------|
| `prog plugin install <name>` | 🔌 | Downloads from marketplace |
| `prog plugin verify <name>` | 🔌 | Signature verification |
| `prog plugin list` | 🔌 | Reads local plugin dir |
| `prog plugin remove <name>` | 🔌 | Local filesystem only |
| `prog trust add <keyid>` | 🔌 | Local trust store |
| `prog trust list` | 🔌 | Local trust store |

---

## Lint Checks

Run these checks in CI or pre-commit to prevent regression:

### Check 1: No new external binary spawns in core
```bash
# Fails if any core source file spawns a non-git subprocess
grep -rn "std::process::Command::new" src/ \
  | grep -v "git" \
  | grep -v "citadel" \
  | grep -v "sober" \
  | grep -v "src/plugins/"
```

### Check 2: No new HTTP clients in core
```bash
# Fails if core gains a direct HTTP dependency
grep -rn "reqwest\|hyper\|curl" src/ \
  | grep -v "src/plugins/" \
  | grep -v "src/marketplace/"
```

### Check 3: Citadel shim boundary intact
```bash
# Verifies command.rs delegates to shim, not inline logic
grep -n '"citadel"' src/command.rs | grep -q "citadel::shim::execute"
```

---

## Migration Backlog

1. **Sync providers** (`src/sync/`) → `sync-gitlab`, `sync-forgejo` plugins
2. **Remote doctor** (`src/remote.rs`) → `sync-*` plugin capability
3. **Review push** (`src/main.rs:451`) → `sync-*` plugin capability
4. **Citadel** (`src/citadel/`) → Standalone premium plugin after SDK Phase 1-3
