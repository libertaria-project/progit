# ProGit

**Terminal-native project management for git repositories**  
Data-first workflow, local-first by default.

[![License: LCL-1.0](https://img.shields.io/badge/License-LCL--1.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.8.2--beta-green.svg)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)

---

## What is ProGit?

ProGit is a terminal-first platform for:

- local issue tracking stored as JSON in `.project/issues/*.json`
- Kanban board and sprint planning
- merge request review workflows and code-level comments
- forge sync (Forgejo/GitLab) on demand
- virtual branches and interactive rebase
- local-first AI actions (Ollama)
- plugin runtime with a marketplace and premium plugins
- release-gate quality checks via Sober integration

No cloud DB. No hidden state. Your data belongs in your repo and git history.

Typical launch path: default to terminal UI (`prog`) and use CLI subcommands for sync, plugins, and admin tasks.

---

## Install

### Recommended installer (Linux/macOS)

```bash
curl -fsSL https://progit.sovereign-society.org/install.sh | sh
```

This installs a released binary and verifies minisign when available.
On most systems it lands in `~/.local/bin` or `~/bin`; if needed, confirm
that one of those paths is on your `PATH`.

### Homebrew (macOS/Linux)

```bash
brew install sovereign-society/tap/progit
```

### Arch Linux (AUR)

```bash
paru -S progit-bin
# or
yay -S progit-bin
```

### From source

```bash
git clone https://git.sovereign-society.org/ProGit/progit
cd progit
cargo build --release
./scripts/link-user-bin.sh target/release/prog
```

`./scripts/link-user-bin.sh` creates `~/bin/prog`, which avoids stale `/usr/local/bin` binaries shadowing your current release.

Verify:

```bash
prog --version
```

---

## Quick start

```bash
cd your-project/      # must be a git repo
prog init             # creates .project/ and .project/issues/
prog                  # launches the TUI
```

Inside the TUI:

- `Tab` — switch modes (`Issues`, `Kanban`, `MRs`, `Dashboard`)
- `j/k` — move selection
- `n` — new issue
- `Ctrl+P` — fuzzy palette (`issues`, `files`, `commits`, `commands`)
- `?` — help
- `q` — quit

---

## Useful CLI

```bash
prog --help
prog sync push            # push local issues to configured forge
prog sync pull            # pull issues from configured forge
prog remote doctor        # pre-flight host checks
prog plugin search <q>    # search marketplace
prog plugin install <name>
prog plugin update
```

Plugins are first-class command namespaces:

```bash
prog plugin sober route list
prog plugin sober-raccoon route list
```

---

## What’s new in 0.8.x

- Plugin commands are now command-chain-native via manifest `contributions`.
- Sober gates are callable from ProGit for deterministic, safer release checks.
- Signed release artifacts and slimmer binary release handling keep the default binary under the 7 MB doctrine budget.
- Source builds are linked to `~/bin/prog` by default.

For release notes and history, read [`CHANGELOG.md`](CHANGELOG.md).

---

## Documentation

- [Website](https://progit.dev)
- [Docs](https://docs.progit.dev)
- [SDK](https://sdk.progit.dev)
- [Plugins](https://plugins.progit.dev)
- [Plugin SDK Guide](docs/PLUGIN_SDK.md)
- [Contributing](CONTRIBUTING.md)

---

## Contributing

Contributions are welcome. Use normal Git workflow and open MRs against the `main` branch.

- [Issue Tracker](https://git.sovereign-society.org/ProGit/progit/issues)
- [Source Repository](https://git.sovereign-society.org/ProGit/progit)

---

## License

- **Core TUI:** [LCL-1.0](legal/LICENSE_COMMONWEALTH.md)
- **Plugin SDK / SDK-facing APIs:** [LSL-1.0](legal/LICENSE_SOVEREIGN.md)
- **Documentation:** [LUL-1.0](legal/LICENSE_UNBOUND.md)
- **Proprietary enterprise support:** [LVL-1.0](legal/LICENSE_VENTURE.md)

---

**Made by [Sovereign Society](https://sovereign-society.org)**
