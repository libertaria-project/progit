# Citadel Integration

**Status:** Infrastructure Complete | Binary Pending

ProGit transforms into an **Infrastructure Cockpit** when `CITADEL.kdl` is detected, providing native integration with the `citadel` infrastructure compiler.

---

## Architecture

```
┌───────────────────────────────────────────────────────────┐
│                      ProGit TUI                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Command Mode │  │  Log Viewer  │  │  Git Status  │     │
│  │   :pano      │  │   (Modal)    │  │   🔱 Icon    │     │
│  └──────┬───────┘  └──────▲───────┘  └──────────────┘     │
│         │                 │                               │
└─────────┼─────────────────┼───────────────────────────────┘
          │                 │
          ▼                 │
┌────────────────────────────────────────────────────────────┐
│              Citadel Module (Rust)                      │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  spawn_validate() → Background Thread                │  │
│  │  spawn_plan()     → Background Thread                │  │
│  │  spawn_apply()    → Background Thread                │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │ mpsc::channel                      │
│                       ▼                                    │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  CitadelEvent → CitadelStatus → App State                  │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────┬──────────────────────────────────────┘
                      │
                      ▼
              ┌───────────────┐
              │ citadel binary│
              │  (subprocess) │
              └───────────────┘
```

---

## Detection & Initialization

### Automatic Detection

ProGit detects Citadel repos via:
1. **Root Discovery:** Walks up from `pwd` looking for `CITADEL.kdl`
2. **Activation:** Sets `app.is_citadel_repo = true`
3. **Visual Indicator:** Shows `🔱` icon in git status bar
4. **Binary Check:** Validates `citadel` is in `PATH` (lazy check, non-fatal)

### Initialization Without Git

ProGit now supports **infrastructure-only repos**:

```bash
# Traditional: Git repo required
cd /my/project
git init
prog

# New: CITADEL.kdl is sufficient
cd /my/infra
touch CITADEL.kdl
prog  # Initializes .project/ and .progit/
```

**Error Message (if neither exists):**
```
❌ No git repository or CITADEL.kdl found.
   ProGit requires either:
   - A git repository (run 'git init'), or
   - A CITADEL.kdl file (infrastructure repo)
```

---

## Commands

All citadel commands are accessed via the command palette (`:` key).

### `:pano validate`

**Purpose:** Validate `CITADEL.kdl` against policy constraints.

**Behavior:**
- Spawns `citadel validate CITADEL.kdl` in background thread
- Non-blocking (TUI remains responsive)
- Shows status in status bar
- Displays policy violations if validation fails

**Example:**
```
:pano validate
→ 🔱 Validation started...
→ ✓ Configuration valid
```

**On Failure:**
```
:pano validate
→ 🔱 Validation started...
→ ✗ POLICY VIOLATION: Network 'prod' exceeds max_instances (10 > 5)
```

---

### `:pano plan [env]`

**Purpose:** Generate infrastructure plan for specified environment.

**Behavior:**
- Spawns `citadel plan --env <env>` in background thread
- **Auto-opens modal log viewer** with streaming output
- Color-coded output:
  - **Green:** Additions (`+`)
  - **Red:** Errors, deletions (`-`)
  - **Yellow:** Changes (`~`)
  - **White:** Informational
- Press `Esc` to close modal

**Example:**
```
:pano plan devnet
→ 🔱 Plan started... Log viewer opened.

┌─────────────────────────────────────────────────────────┐
│ 🔱 Planning devnet environment...                      │
├─────────────────────────────────────────────────────────┤
│ Terraform will perform the following actions:          │
│                                                         │
│   + resource "google_compute_instance" "validator-01"  │
│   ~ resource "google_compute_network" "devnet"         │
│                                                         │
│ Plan: 1 to add, 1 to change, 0 to destroy.            │
│                                                         │
│ [Esc] Close                                            │
└─────────────────────────────────────────────────────────┘
```

**Default Environment:** `devnet`

---

### `:pano status`

**Purpose:** Check current citadel operation status.

**Behavior:**
- Shows current `CitadelStatus` (Idle, Running, Success, Error)
- Useful for checking background operation progress

**Example:**
```
:pano status
→ 🔱 Planning devnet...  (if running)
→ 🔱 Citadel: Idle    (if idle)
```

---

### `:pano apply` (Disabled)

**Purpose:** Apply infrastructure changes.

**Current Behavior:**
```
:pano apply devnet
→ ⚠️ Apply disabled in command mode for safety. Use dedicated Ops interface.
```

**Rationale:** `apply` is intentionally disabled in command mode to prevent accidental infrastructure changes. Will be enabled via dedicated Ops Tab UI in Phase 2.

---

## Configuration

### Custom Binary Path

By default, ProGit looks for `citadel` in `PATH`. To specify a custom path:

**`.project/config.kdl`:**
```kdl
citadel {
    binary-path "/opt/pano-forge/bin/citadel"
}
```

**App State:**
```rust
app.citadel_binary_path = Some("/opt/pano-forge/bin/citadel".to_string());
```

---

## Event Flow

### Validation Flow

```
User: :pano validate
  ↓
Command Handler
  ↓ (dispatch)
spawn_validate(repo_path, binary_path, sender)
  ↓ (background thread)
citadel validate CITADEL.kdl
  ↓ (exit code + stderr)
CitadelEvent::ValidationComplete { success, message }
  ↓ (mpsc channel)
Main Loop Event Polling
  ↓
app.citadel_status = Success/Error
  ↓
Status Bar Update
```

### Plan Flow (with Modal)

```
User: :pano plan devnet
  ↓
Command Handler
  ↓ (set state)
app.show_citadel_log = true
app.citadel_output.clear()
  ↓ (dispatch)
spawn_plan(repo_path, env, binary_path, sender)
  ↓ (background thread)
citadel plan --env devnet
  ↓ (stdout line-by-line)
CitadelEvent::Status(OutputLine("..."))
  ↓ (mpsc channel)
Main Loop Event Polling
  ↓
app.citadel_output.push(line)
  ↓
Modal Renders Live Output
  ↓ (on completion)
CitadelEvent::PlanComplete { success, output }
  ↓
Status Bar: "✓ Plan completed successfully"
```

---

## Non-Blocking Guarantee

**Critical Design Principle:** All `citadel` subprocess calls are **non-blocking**.

### Why This Matters

Infrastructure operations can take 10-60 seconds:
- Terraform state refresh from cloud APIs
- OpenTofu plan generation
- Policy validation against large configs

**Without non-blocking execution:**
```
User presses :pano plan
→ TUI freezes for 30 seconds
→ No keyboard input registered
→ User thinks app crashed
→ Ctrl+C kills entire session
```

**With non-blocking execution:**
```
User presses :pano plan
→ "🔱 Plan started..." appears immediately
→ TUI remains responsive
→ User can navigate, view issues, etc.
→ Modal shows streaming output
→ "✓ Plan completed" when done
```

### Implementation

**Pattern:**
```rust
// ❌ WRONG: Blocks TUI
let output = Command::new("citadel").output()?;

// ✅ CORRECT: Non-blocking
std::thread::spawn(move || {
    let output = Command::new("citadel").output();
    sender.send(CitadelEvent::PlanComplete { ... });
});
```

**Event Polling (main loop):**
```rust
loop {
    // Poll for async results (non-blocking)
    if let Some(rx) = app.citadel_event_rx.take() {
        while let Ok(event) = rx.try_recv() {
            // Update app state
        }
        app.citadel_event_rx = Some(rx);
    }
    
    // Render TUI
    terminal.draw(|f| render(f, &mut app))?;
    
    // Handle input
    if event::poll(Duration::from_millis(100))? {
        // ...
    }
}
```

---

## Testing Without `citadel`

Since `citadel` is pre-alpha, you can test the integration with a mock binary:

### Create Mock Binary

**`/usr/local/bin/citadel`:**
```bash
#!/bin/bash
case "$1" in
    validate)
        echo "✓ Configuration valid"
        exit 0
        ;;
    plan)
        echo "Terraform will perform the following actions:"
        echo ""
        echo "  + resource \"google_compute_instance\" \"validator-01\""
        echo "  ~ resource \"google_compute_network\" \"devnet\""
        echo ""
        echo "Plan: 1 to add, 1 to change, 0 to destroy."
        sleep 2  # Simulate slow operation
        exit 0
        ;;
    *)
        echo "Unknown command: $1"
        exit 1
        ;;
esac
```

```bash
chmod +x /usr/local/bin/citadel
```

### Test Flow

```bash
cd /path/to/citadel/repo
prog

# In TUI:
:pano validate
# → Should show "✓ Configuration valid"

:pano plan devnet
# → Modal opens with streaming output
# → Press Esc to close
```

---

## Phase 2 Roadmap

**Deferred Features:**

1. **Pre-Commit Guillotine**
   - Auto-validate on `CITADEL.kdl` staged changes
   - Block commit if policy violations detected
   - Visual feedback in TUI

2. **Dedicated Ops Tab**
   - New `ViewMode::Infrastructure`
   - Plan/Apply buttons with confirmation dialogs
   - Full-screen output console
   - SOPS secret handling (env injection)

3. **Hexahedron Visualizer**
   - Interactive tree view of KDL structure
   - Navigate cells/networks/regions
   - Syntax highlighting

4. **Real-Time KDL Validation**
   - Live syntax checking as you edit
   - Inline error markers
   - Auto-completion

---

## Troubleshooting

### "Unknown command: pano"

**Cause:** Binary not rebuilt after code changes.

**Fix:**
```bash
cd /path/to/progit
cargo clean
cargo build --release
./target/release/prog
```

### "citadel binary not found in PATH"

**Cause:** `citadel` not installed or not in `PATH`.

**Fix:**
```bash
# Check if citadel exists
which citadel

# If not, either:
# 1. Install citadel (when available)
# 2. Create mock binary (see "Testing Without citadel")
# 3. Configure custom path in .project/config.kdl
```

### "Not a Citadel repo"

**Cause:** No `CITADEL.kdl` found in current directory or parent directories.

**Fix:**
```bash
# Verify file exists
ls -la CITADEL.kdl

# If in subdirectory, cd to repo root
cd /path/to/repo/with/CITADEL.kdl
prog
```

### Modal doesn't open for `:pano plan`

**Cause:** `app.show_citadel_log` not being set.

**Fix:** Rebuild binary (see "Unknown command: pano" above).

---

## Code Reference

**Key Files:**
- `src/citadel/mod.rs` - Core async module
- `src/tui/widget_citadel_log.rs` - Modal log viewer
- `src/command.rs` - Command palette integration
- `src/tui/input.rs` - Keyboard handling
- `src/main.rs` - Event polling loop

**Key Functions:**
- `citadel::spawn_validate()` - Background validation
- `citadel::spawn_plan()` - Background plan with streaming
- `citadel::is_citadel_repo()` - Detection logic
- `widget_citadel_log::render()` - Modal rendering

**Key State:**
- `app.is_citadel_repo: bool` - Detection flag
- `app.citadel_status: CitadelStatus` - Current operation status
- `app.citadel_output: Vec<String>` - Streaming output buffer
- `app.show_citadel_log: bool` - Modal visibility
- `app.citadel_event_rx: Receiver<CitadelEvent>` - Async event channel

---

## License

Same as ProGit: LCL-1.0
