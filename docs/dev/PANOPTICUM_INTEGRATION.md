# Panopticum Integration - Developer Notes

**For Future Development & Maintenance**

---

## Quick Start (When Resuming)

1. **Verify panoctl binary exists:**
   ```bash
   which panoctl
   panoctl --version
   ```

2. **Test integration:**
   ```bash
   cd /path/to/panopticum/repo
   cargo run --bin prog
   # Press : then type: pano validate
   ```

3. **If issues, rebuild:**
   ```bash
   cargo clean && cargo build
   ```

---

## Architecture Decisions

### Why Non-Blocking?

**Problem:** Infrastructure operations (Terraform, OpenTofu) can take 10-60 seconds. Blocking the TUI during this time creates a terrible UX.

**Solution:** All `panoctl` calls spawn in background threads and communicate via `mpsc::channel`.

**Trade-offs:**
- ✅ TUI stays responsive
- ✅ User can continue working
- ✅ Real-time output streaming
- ❌ Slightly more complex code (channels, event polling)
- ❌ Must handle thread lifecycle

**Verdict:** Complexity is worth it. Frozen UI is unacceptable for infrastructure tooling.

---

## Event Channel Pattern

### Flow

```rust
// 1. Create channel (once, at startup)
let (tx, rx) = mpsc::channel();
app.pano_event_tx = Some(tx);
app.pano_event_rx = Some(rx);

// 2. Spawn background job
let sender = app.pano_event_tx.clone().unwrap();
panopticum::spawn_plan(repo_path, env, binary_path, sender);

// 3. Background thread sends events
thread::spawn(move || {
    sender.send(PanoEvent::Status(Running("Planning...")));
    // ... run panoctl ...
    sender.send(PanoEvent::PlanComplete { success, output });
});

// 4. Main loop polls for events (non-blocking)
if let Some(rx) = app.pano_event_rx.take() {
    while let Ok(event) = rx.try_recv() {
        match event {
            PanoEvent::Status(status) => app.pano_status = status,
            // ...
        }
    }
    app.pano_event_rx = Some(rx); // Put back
}
```

### Why `.take()` Instead of `ref`?

**Borrow Checker Issue:**
```rust
// ❌ WRONG: Immutable borrow prevents mutation
if let Some(ref rx) = app.pano_event_rx {
    while let Ok(event) = rx.try_recv() {
        app.set_status(...);  // ERROR: can't mutate app
    }
}

// ✅ CORRECT: Take ownership, mutate, put back
if let Some(rx) = app.pano_event_rx.take() {
    while let Ok(event) = rx.try_recv() {
        app.set_status(...);  // OK: app is mutable
    }
    app.pano_event_rx = Some(rx);
}
```

---

## Modal Log Viewer

### Why Modal Instead of New ViewMode?

**Options Considered:**
1. **New ViewMode::Infrastructure** - Full-screen dedicated view
2. **Status Bar Only** - Show truncated output
3. **Modal Overlay** - Centered popup

**Decision:** Modal overlay (Phase 1), dedicated view (Phase 2).

**Rationale:**
- Modal is faster to implement (single widget)
- Doesn't disrupt existing view navigation
- User can quickly check plan and return to work
- Dedicated view can be added later for complex operations

### Rendering Order

**Critical:** Modal must render **last** to appear on top.

```rust
// src/tui.rs render() function
pub fn render(frame: &mut Frame, app: &mut App) -> UIAreas {
    // ... render main content ...
    
    // Render overlays (order matters!)
    if app.input_mode == InputMode::DetailView { ... }
    if app.input_mode == InputMode::Settings { ... }
    if app.input_mode == InputMode::FuzzyPalette { ... }
    
    // Panopticum modal LAST (top layer)
    if app.show_pano_log {
        widget_pano_log::render(frame, app);
    }
    
    areas
}
```

---

## PANOPTICUM.kdl Detection

### Root Discovery Algorithm

```rust
fn find_project_root() -> Result<PathBuf> {
    let current = std::env::current_dir()?;
    let mut path = current.as_path();
    
    loop {
        // Check for markers (order matters!)
        if path.join(".git").exists()           // Git repo
            || path.join(".project").exists()    // ProGit workspace
            || path.join("PANOPTICUM.kdl").exists() {  // Infra repo
            return Ok(path.to_path_buf());
        }
        
        // Walk up
        match path.parent() {
            Some(parent) => path = parent,
            None => break,
        }
    }
    
    // Fallback: current directory
    Ok(current)
}
```

### Why This Works

**Scenario 1: Git repo with PANOPTICUM.kdl**
```
/my/project/
├── .git/
├── PANOPTICUM.kdl
└── cells/
    └── 20-validator/
        └── (you are here)
```
→ Finds `/my/project/` via `.git` (stops at first match)

**Scenario 2: Infrastructure-only repo**
```
/my/infra/
├── PANOPTICUM.kdl
└── cells/
    └── 20-validator/
        └── (you are here)
```
→ Finds `/my/infra/` via `PANOPTICUM.kdl`

**Scenario 3: Nested structure**
```
/workspace/
├── PANOPTICUM.kdl
└── projects/
    └── app1/
        ├── .git/
        └── (you are here)
```
→ Finds `/workspace/projects/app1/` via `.git` (closest match wins)

---

## Command Palette Integration

### Why `:` Key?

**Vim Convention:** `:` enters command mode in Vim/Neovim.

**ProGit Users:** Likely familiar with Vim keybindings (`j`/`k` navigation, etc.).

**Consistency:** ProGit already uses `:` for commands like `:theme`, `:rebase`, `:diff`.

### Adding `:` to All View Modes

**Bug:** Initially, `:` only worked in `ViewMode::List`.

**Fix:** Added handler to `handle_kanban_key()` and `handle_mr_list_key()`.

```rust
// src/tui/input.rs
fn handle_kanban_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        // ... other keys ...
        
        // Command Palette
        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command;
            app.command_input.clear();
            KeyAction::Refresh
        }
        
        // ...
    }
}
```

**Lesson:** When adding global commands, check **all** view mode handlers.

---

## Binary Path Configuration

### Current Implementation

**Default:** Look for `panoctl` in `PATH`.

**Custom Path:** (Planned, not yet implemented)
```kdl
// .project/config.kdl
panopticum {
    binary-path "/opt/pano-forge/bin/panoctl"
}
```

### How to Implement Custom Path

1. **Parse config:**
   ```rust
   // src/storage/config.rs
   #[derive(Deserialize)]
   pub struct PanopticumConfig {
       pub binary_path: Option<String>,
   }
   
   #[derive(Deserialize)]
   pub struct Config {
       pub panopticum: Option<PanopticumConfig>,
       // ...
   }
   ```

2. **Set app state:**
   ```rust
   // src/main.rs run_app()
   if let Some(pano_config) = config.panopticum {
       app.panoctl_binary_path = pano_config.binary_path;
   }
   ```

3. **Use in spawn functions:**
   ```rust
   // Already implemented!
   let binary = binary_path.unwrap_or("panoctl");
   Command::new(binary).args(...).spawn()?;
   ```

---

## Testing Strategy

### Unit Tests

**Location:** `src/panopticum/mod.rs`

**Coverage:**
- ✅ `is_panopticum_repo()` - Detection logic
- ✅ `get_binary_path()` - Path resolution
- ✅ `create_event_channel()` - Channel creation

**Missing (add when panoctl is available):**
- Integration tests with real `panoctl` binary
- Mock subprocess tests
- Error handling tests

### Manual Testing Checklist

**Without panoctl:**
- [ ] `:pano validate` shows "binary not found" error
- [ ] `:pano plan` shows "binary not found" error
- [ ] No crashes or panics

**With mock panoctl:**
- [ ] `:pano validate` shows success/failure
- [ ] `:pano plan` opens modal
- [ ] Modal shows streaming output
- [ ] Esc closes modal
- [ ] TUI remains responsive during operation

**With real panoctl (future):**
- [ ] Validation detects policy violations
- [ ] Plan shows actual Terraform diff
- [ ] Long operations don't freeze UI
- [ ] Errors are displayed correctly

---

## Known Limitations

### 1. No Scrolling in Modal

**Issue:** If `panoctl plan` output exceeds modal height, content is truncated.

**Workaround:** Modal shows last N lines (fits in viewport).

**Fix (Phase 2):** Add scrolling support with `j`/`k` keys.

### 2. No Apply Confirmation Dialog

**Issue:** `:pano apply` is completely disabled.

**Rationale:** Infrastructure changes are dangerous. Need proper confirmation UI.

**Fix (Phase 2):** Dedicated Ops Tab with:
- Plan preview
- Confirmation checkbox
- "Apply" button (requires explicit click)

### 3. No SOPS Integration

**Issue:** Secrets aren't decrypted/injected.

**Rationale:** SOPS handling requires careful design (never log secrets, in-memory only).

**Fix (Phase 2):** 
```rust
// Decrypt SOPS file
let secrets = sops::decrypt("secrets.yaml")?;

// Inject as env vars to subprocess
Command::new("panoctl")
    .env("VAULT_TOKEN", secrets.get("vault_token"))
    .spawn()?;
```

### 4. No Pre-Commit Hook

**Issue:** Validation doesn't run automatically on commit.

**Rationale:** Git hook installation is invasive. Need user consent.

**Fix (Phase 2):**
- Detect staged `PANOPTICUM.kdl` changes
- Run validation automatically
- Block commit if violations found
- Provide override mechanism (`--no-verify`)

---

## Future Enhancements

### Phase 2: Ops Tab

**Goal:** Dedicated infrastructure management interface.

**Features:**
- Full-screen plan/apply view
- Side-by-side diff (current vs. planned)
- Resource graph visualization
- Apply confirmation workflow
- Rollback support

**UI Mockup:**
```
┌─────────────────────────────────────────────────────────────┐
│ 🔱 Infrastructure Operations                                │
├─────────────────────────────────────────────────────────────┤
│ Environment: [devnet ▼]                    [Plan] [Apply]  │
├─────────────────────────────────────────────────────────────┤
│ Current State          │ Planned Changes                    │
├────────────────────────┼────────────────────────────────────┤
│ validator-01 (running) │ + validator-02 (new)              │
│ network-devnet (active)│ ~ network-devnet (modify)         │
│                        │                                    │
│                        │ Plan: 1 to add, 1 to change       │
│                        │                                    │
│                        │ [ ] I have reviewed this plan     │
│                        │ [Apply Changes]                   │
└────────────────────────┴────────────────────────────────────┘
```

### Phase 3: Hexahedron Visualizer

**Goal:** Interactive KDL structure browser.

**Features:**
- Tree view of cells/networks/regions
- Expand/collapse nodes
- Jump to file location
- Syntax highlighting
- Validation status per node

**UI Mockup:**
```
┌─────────────────────────────────────────────────────────────┐
│ 🔱 Hexahedron Structure                                     │
├─────────────────────────────────────────────────────────────┤
│ ▼ networks                                                  │
│   ▼ devnet                                                  │
│     ├─ regions: ["us-central1"]                            │
│     ├─ max_instances: 5                                    │
│     └─ fleets                                               │
│         ▼ validators                                        │
│           ├─ count: 3                                       │
│           └─ machine_type: "n2-standard-4"                 │
│   ▶ testnet                                                 │
│   ▶ mainnet                                                 │
│ ▶ cells                                                     │
│   ▶ 00-bedrock                                              │
│   ▶ 20-validator                                            │
└─────────────────────────────────────────────────────────────┘
```

---

## Maintenance Notes

### When panoctl CLI Changes

**If panoctl adds new commands:**
1. Add case to `src/command.rs` match statement
2. Add `spawn_*()` function to `src/panopticum/mod.rs`
3. Add event variant to `PanoEvent` enum
4. Update docs

**If panoctl changes output format:**
1. Update color-coding logic in `src/tui/widget_pano_log.rs`
2. Update parsing logic in event handlers
3. Test with real output

**If panoctl changes exit codes:**
1. Update success detection in `spawn_*()` functions
2. Update error messages

### When ProGit Refactors

**If command system changes:**
- Update `src/command.rs` to match new pattern
- Ensure `:pano` commands still work

**If TUI rendering changes:**
- Verify modal still renders on top
- Check z-index/layer ordering

**If event loop changes:**
- Verify event polling still works
- Check for borrow checker issues

---

## Performance Considerations

### Channel Overhead

**Concern:** Does `mpsc::channel` add latency?

**Answer:** Negligible. `try_recv()` is non-blocking and very fast (~nanoseconds).

**Measurement:**
```rust
let start = Instant::now();
while let Ok(event) = rx.try_recv() { ... }
let elapsed = start.elapsed();
// Typical: <1ms for 1000 events
```

### Thread Spawning

**Concern:** Does spawning threads for each command cause issues?

**Answer:** No. Threads are lightweight and short-lived.

**Typical Usage:**
- User runs `:pano plan` → 1 thread spawned
- Thread runs for 5-30 seconds
- Thread exits, resources freed
- Rare to have >1 concurrent operation

**If Concerned:** Could use a thread pool, but overkill for current usage.

### Output Buffer Size

**Concern:** What if `panoctl plan` outputs 10,000 lines?

**Answer:** `Vec<String>` can handle it, but modal won't show all.

**Current Limit:** Modal shows last ~50 lines (fits viewport).

**Future:** Add scrolling or pagination.

---

## Security Considerations

### Subprocess Injection

**Risk:** User-controlled input passed to `Command::new()`.

**Mitigation:** 
- Binary path is from config (trusted) or PATH (system)
- Arguments are hardcoded (`validate`, `plan`, `--env`)
- Environment name is validated (alphanumeric only)

**Code:**
```rust
// ✅ SAFE: Binary path from config or PATH
let binary = get_binary_path(custom_path);

// ✅ SAFE: Hardcoded arguments
Command::new(binary).args(["plan", "--env", env])

// ❌ UNSAFE: User input directly in args
Command::new("panoctl").arg(user_input)  // DON'T DO THIS
```

### Secret Handling

**Risk:** Secrets logged or displayed in TUI.

**Mitigation (Phase 2):**
- SOPS decryption in-memory only
- Secrets passed as env vars (not args)
- Never log env vars
- Clear secrets from memory after use

**Code Pattern:**
```rust
// Decrypt SOPS
let secrets = sops::decrypt_in_memory("secrets.yaml")?;

// Inject as env (not visible in process list)
let mut cmd = Command::new("panoctl");
for (key, value) in secrets {
    cmd.env(key, value);
}

// DON'T log command
// log::debug!("{:?}", cmd);  // Would expose secrets!

// Clear secrets
drop(secrets);
```

---

## Debugging Tips

### Enable Verbose Logging

```bash
RUST_LOG=debug cargo run --bin prog
```

**Logs to check:**
- `🔱 Panopticum mode activated` - Detection worked
- `⚠️ panoctl binary not found` - Binary missing
- Thread spawn/exit messages

### Check Event Channel

**Add debug prints:**
```rust
// src/main.rs event polling
if let Some(rx) = app.pano_event_rx.take() {
    while let Ok(event) = rx.try_recv() {
        eprintln!("DEBUG: Received event: {:?}", event);  // ADD THIS
        match event { ... }
    }
    app.pano_event_rx = Some(rx);
}
```

### Verify Binary Execution

**Test panoctl directly:**
```bash
panoctl validate PANOPTICUM.kdl
echo $?  # Check exit code
```

**Strace subprocess:**
```bash
strace -f -e execve cargo run --bin prog 2>&1 | grep panoctl
```

---

## Contact & Support

**Integration Author:** Voxis (AI Agent)  
**Date:** 2025-12-11  
**ProGit Version:** 0.2.0  
**Status:** Phase 1 Complete, Awaiting panoctl Alpha

**For Issues:**
1. Check `PANOPTICUM.md` user docs
2. Review this developer guide
3. Check git history for context
4. Test with mock `panoctl` binary first
