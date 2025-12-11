# Contributing to ProGit

Thank you for considering contributing to ProGit! 🎉

## Development Workflow

1. **Fork & Clone**
2. **Create feature branch**: `git checkout -b feature/amazing-feature`
3. **Make changes**
4. **Test**: `cargo test && cargo clippy`
5. **Commit**: Use descriptive messages (see [Conventional Commits](#conventional-commits))
6. **Push & PR**

## Branching Strategy

ProGit uses a **two-branch model**. See [docs/BRANCHING.md](docs/BRANCHING.md) for full details.

### Branches

- **`main`** - Stable releases only (tagged)
- **`develop`** - Active development (all work lands here first)
- **`feature/*`** - Individual features/fixes

### Workflow

```bash
# Start new feature
git checkout develop
git checkout -b feature/my-feature

# Make changes with conventional commits
git commit -m "feat(tui): add new widget"

# Merge to develop
git checkout develop
git merge feature/my-feature --no-ff
git push origin develop
```

### Conventional Commits

**All commits must follow this format:**

```
<type>(<scope>): <description>
```

**Types:**
- `feat:` - New feature (bumps MINOR version)
- `fix:` - Bug fix (bumps PATCH version)
- `docs:` - Documentation only
- `style:` - Code formatting
- `refactor:` - Code refactoring
- `test:` - Adding tests
- `chore:` - Maintenance

**Examples:**
```bash
feat(sync): add GitHub provider support
fix(tui): prevent crash on empty issue list
docs: update installation instructions
```

### Releasing

**Automated version bumping:**

```bash
# Analyze commits and bump version
./scripts/bump-version.sh

# Create release (merges develop → main, tags, pushes)
./scripts/release.sh
```

See [docs/BRANCHING.md](docs/BRANCHING.md) for complete release workflow.

## Code Style

**ProGit follows strict Rust conventions:**

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Write tests for new features
- Update README if adding user-facing features

## Architecture Principles

### PANOPTICUM Standard

Every feature follows the **Sovereign Index** pattern:

```
src/
├── feature.rs          # Index (public API)
└── feature/            # Implementation
    ├── model.rs
    ├── operations.rs
    └── tests.rs
```

**Rules:**
1. One feature per folder
2. Index exports public API
3. Implementation stays private
4. Tests live with implementation

### Code Standards

```rust
// ✅ Good
pub fn update_due(issue: &Issue, due: Option<DateTime<Utc>>) -> Issue {
    Issue {
        due,
        updated: Utc::now(),
        ..issue.clone()
    }
}

// ❌ Bad (mutating in place)
pub fn update_due(issue: &mut Issue, due: Option<DateTime<Utc>>) {
    issue.due = due;
    issue.updated = Utc::now();
}
```

**Why immutable?** Explicit data flow, easier testing, no side effects.

## Feature Areas

### Core (`src/issue/`)
- Issue model and domain logic
- Keep business rules here
- No I/O or UI concerns

### Storage (`src/storage/`)
- KDL and JSON persistence
- Migration logic
- File system operations

### Sync (`src/sync/`)
- Remote forge adapters (GitLab, Forgejo, GitHub)
- OAuth/token management
- Merge conflict resolution

### TUI (`src/tui/`)
- Ratatui widgets
- Input handling (keyboard + mouse)
- Themes

## Adding a New Sync Provider

Example: Adding GitHub support

1. **Create** `src/sync/github.rs`:

```rust
use super::SyncProvider;
use crate::issue::Issue;
use anyhow::Result;

pub struct GitHubProvider {
    config: SyncConfig,
    client: Client,
}

impl SyncProvider for GitHubProvider {
    fn login(&self) -> Result<()> { todo!() }
    fn pull(&self) -> Result<Vec<Issue>> { todo!() }
    fn push(&self, issues: &mut [Issue]) -> Result<()> { todo!() }
    fn delete_missing(&self, local: &[Issue]) -> Result<usize> { todo!() }
}
```

2. **Register** in `src/sync/mod.rs`:

```rust
pub mod github;

pub fn create_provider(config: SyncConfig) -> Box<dyn SyncProvider> {
    match config.provider.as_str() {
        "github" => Box::new(github::GitHubProvider::new(config)),
        // ...
    }
}
```

3. **Test** with real API (use your own repo):

```bash
# .project/config.kdl
sync {
    provider "github"
    url "https://api.github.com"
    owner "yourname"
    repo "test-repo"
}
```

## Testing

```bash
# Unit tests
cargo test

# Integration tests (requires Git repo)
cd /tmp/test-repo
git init
prog  # Should auto-initialize

# Manual TUI testing
cargo run
```

## Documentation

- **Code comments** - Explain WHY, not WHAT
- **Doc comments** - `///` for public APIs
- **README updates** - For user-facing features
- **Architecture docs** - For design decisions

## Pull Request Guidelines

**Good PR:**
- Single focused change
- Tests included
- README updated (if needed)
- Clean commit history
- Descriptive title and description

**Great PR:**
- Includes before/after screenshots (for UI)
- Performance benchmarks (if relevant)
- Migration guide (for breaking changes)

## Bug Reports

**Use GitHub Issues with:**
- ProGit version (`prog --version`)
- OS and terminal
- Steps to reproduce
- Expected vs actual behavior
- Logs/screenshots

## Feature Requests

**We love ideas!** But please:
- Check existing issues first
- Explain the use case
- Suggest implementation if you can
- Be patient - we're a small team

## Code of Conduct

**Be excellent to each other.**

- Respectful communication
- Constructive feedback
- Assume good intentions
- Focus on code, not people

## Questions?

Open a GitHub Discussion or ping on Matrix/Discord (links TBD).

---

**Thank you for making ProGit better!** 🚀
