# Contributing to ProGit

First off, thank you for considering contributing to ProGit! 🎉

Following these guidelines helps communicate that you respect the time of the developers managing and developing this open source project. In return, they should reciprocate that respect in addressing your issue, assessing changes, and helping you finalize your pull requests.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Code Style](#code-style)
- [Testing](#testing)
- [Documentation](#documentation)
- [Community](#community)

## 📜 Code of Conduct

This project adheres to the **Voxis Forge Doctrine** which emphasizes:

- **Brutal Honesty**: Direct, constructive feedback
- **Technical Excellence**: Correctness > Speed
- **Pragmatic Efficiency**: ROI-focused development
- **Mutual Respect**: Professional collaboration

In short: Be direct, be constructive, prioritize quality.

## 🚀 Getting Started

### Types of Contributions

We welcome many types of contributions:

- **🐛 Bug Reports**: Found a bug? Let us know!
- **✨ Feature Requests**: Have an idea? Propose it!
- **📝 Documentation**: Improve guides and examples
- **🔧 Code**: Fix bugs or implement features
- **🎨 Design**: UI/UX improvements
- **🧪 Testing**: Write tests, find edge cases
- **🔌 Plugins**: Create community plugins

### First Time Contributors

Look for issues labeled `good-first-issue` in the [issue tracker](https://git.sovereign-society.org/ProGit/progit/issues). These are specifically curated for newcomers.

## 🛠️ Development Setup

### Prerequisites

- **Rust 1.75+** (`rustup` recommended)
- **Git 2.30+**
- **Linux/BSD** (macOS/Windows support coming)
- **Optional:** Ollama for AI features

### Clone and Build

```bash
# Clone the repository
git clone https://git.sovereign-society.org/ProGit/progit
cd progit

# Build in debug mode (fast compilation)
cargo build

# Run the local quality gate
./scripts/check.sh

# Run the TUI
cargo run

# Build optimized release
cargo build --release
```

### Project Structure

```
progit/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library exports
│   ├── tui/                 # Terminal UI (EUPL-1.2)
│   │   ├── app.rs           # Application state
│   │   ├── input.rs         # Keyboard/mouse handling
│   │   ├── widget_*.rs      # UI components
│   │   └── agent_executor.rs  # AI agent logic
│   ├── virtual_branch.rs    # Virtual branch system
│   ├── agent/               # AI integration
│   │   ├── context.rs       # Context gathering
│   │   └── ollama.rs        # Ollama client
│   ├── plugins/             # Plugin system (Apache-2.0)
│   │   ├── sdk.rs           # Plugin SDK traits
│   │   └── lua_engine.rs    # LuaJIT runtime (WIP)
│   ├── storage/             # Data persistence
│   ├── git/                 # Git operations
│   └── sync/                # Remote sync providers
├── docs/                    # Documentation
├── examples/                # Example code
└── tests/                   # Integration tests
```

## 🌿 Branch Strategy

ProGit uses a three-branch model:

```
forge  →  main  →  stable
 (dev)    (integration)  (releases)
```

| Branch | Purpose | Merges Into |
|--------|---------|-------------|
| `stable` | Tagged releases only. Always deployable. What users download and build from. | — |
| `main` | Integration branch. Reviewed, tested code. PRs land here. | `stable` (on release) |
| `forge` | Agent and contributor landing zone. Work-in-progress, worktree branches, and experimental work merge here first. | `main` (after review) |

### How it works

**For contributors:**
1. Branch off `forge` (or `main` for hotfixes)
2. Do your work, push your branch
3. Open a merge request targeting `forge`
4. After review, it gets merged to `forge`
5. Maintainers promote tested work from `forge` to `main`

**For AI agents (Claude Code, etc.):**
- Agents work in isolated git worktrees
- Worktree branches target `forge`
- After validation, merged to `forge` automatically
- Maintainers review and promote to `main`

**For releases:**
- When `main` is stable and tested, it gets merged to `stable`
- A version tag is created on `stable` (e.g., `v0.7.0-alpha`)
- Only tagged commits on `stable` are considered official releases

**Rules:**
- Never push directly to `stable` – it only receives merges from `main`
- Never push untested code to `main` – use `forge` for work-in-progress
- All commits follow [Conventional Commits 1.0.0](https://www.conventionalcommits.org/)

```bash
# Start new work
git fetch origin
git checkout -b feat/my-feature origin/forge

# When done
git push origin feat/my-feature
# Open MR targeting forge
```

---

## 🎯 How to Contribute

### Bug Reports

**Before submitting:**
1. Search existing issues
2. Try latest `forge` branch
3. Gather reproduction steps

**When submitting:**
```markdown
**Environment:**
- OS: [e.g., Arch Linux]
- ProGit version: [e.g., 0.4.0-alpha]
- Rust version: [e.g., 1.75.0]

**Steps to Reproduce:**
1. ...
2. ...

**Expected Behavior:**
...

**Actual Behavior:**
...

**Logs:**
```bash
prog --debug 2>&1 | tee progit-debug.log
```
```

### Feature Requests

**Template:**
```markdown
**Problem:**
What problem does this solve?

**Proposed Solution:**
Describe your ideal solution

**Alternatives:**
What alternatives have you considered?

**Additional Context:**
Mockups, examples, related issues
```

### Pull Requests

**Workflow:**

1. **Branch off `forge`**
   ```bash
   git fetch origin
   git checkout -b feat/your-feature origin/forge
   ```

2. **Make Changes**
   - Follow code style (see below)
   - Add tests
   - Update docs

3. **Commit**
   ```bash
   # Use Conventional Commits
   git commit -m "feat(virtual-branch): add hunk splitting"
   ```

4. **Push & MR**
   ```bash
   git push origin feat/your-feature
   ```
   - Open merge request targeting `forge`
   - Link related issues
   - Request review

**PR Checklist:**
- [ ] Local quality gate passes (`./scripts/check.sh`)
- [ ] Formatted (`cargo fmt`)
- [ ] Documentation updated
- [ ] Changelog entry added
- [ ] Follows code style

## 📐Code Style

We follow the [**Voxis Forge Rust Coding Standards**](docs/CODING_STANDARDS.md):

### General Principles

1. **MUST be fully optimized**
   - Maximize algorithmic efficiency (O(n) > O(n²))
   - Use parallelization/SIMD where appropriate
   - Follow DRY principle

2. **Correctness > Speed**
   - Never compromise correctness for velocity
   - Explicit error handling (no `unwrap()` in production)
   - Comprehensive testing

3. **Clean Code**
   - Single responsibility per function (<20 lines ideal)
   - Explicit type annotations
   - Descriptive names (no abbreviations)

### Naming Conventions

- **PascalCase**: Modules, Classes, Enums
- **snake_case**: Variables, Functions
- **SCREAMING_SNAKE_CASE**: Constants

### Error Handling

```rust
// ❌ BAD: unwrap() in production
let value = some_option.unwrap();

// ✅ GOOD: Explicit error handling
let value = some_option.ok_or_else(|| {
    Error::MissingValue("Expected value here")
})?;
```

### Documentation

```rust
/// Calculate total cost including tax.
///
/// # Arguments
///
/// * `items` - Slice of items with price fields
/// * `tax_rate` - Tax rate as decimal (0.08 = 8%)
///
/// # Returns
///
/// Total cost with tax applied
///
/// # Examples
///
/// ```
/// let items = vec![Item { price: 10.0 }];
/// let total = calculate_total(&items, 0.08)?;
/// assert_eq!(total, 10.80);
/// ```
pub fn calculate_total(items: &[Item], tax_rate: f64) -> Result<f64> {
    // Implementation
}
```

## 🧪 Testing

### Test Categories

1. **Unit Tests** - Test individual functions
2. **Integration Tests** - Test system interactions
3. **TUI Tests** - Test UI behavior (limited)

### Running Tests

```bash
# Local quality gate
./scripts/check.sh

# All tests
cargo test

# Specific test
cargo test virtual_branch

# With output
cargo test -- --nocapture

# Integration tests only
cargo test --test '*'
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_branch_creation() {
        // Arrange
        let manager = VirtualBranchManager::new(repo_path);
        
        // Act
        let branch = manager.create_branch("feature-1", "").unwrap();
        
        // Assert
        assert_eq!(branch.name, "feature-1");
        assert_eq!(manager.list().len(), 1);
    }
}
```

## 📚 Documentation

### Types of Documentation

1. **Code Comments**: Explain "why", not "what"
2. **Rustdoc**: Public API documentation
3. **Guides**: User-facing docs in `docs/`
4. **Examples**: Working code in `examples/`

### Semantic Comment Tags

```rust
// [HAZMAT] Dangerous code - don't touch without tests
// [SEC] Security critical - crypto/auth/zero-trust
// [COMPLY:GDPR-Art-17] Regulatory requirement
// [ARCH] Architectural decision record
// [DEBT] Technical debt - plan to fix
```

### Documentation Checklist

- [ ] All public functions have docstrings
- [ ] Examples compile and run
- [ ] Guides updated for new features
- [ ] Changelog entry added

## 👥 Community

### Communication Channels

- **GitHub Issues**: Bug reports, feature requests
- **GitHub Discussions**: Questions, ideas
- **Discord**: Real-time chat, community support
- **Twitter**: Announcements, tips

### Getting Help

1. **Check Documentation**: `docs/` folder
2. **Search Issues**: Someone may have asked before
3. **Ask in Discord**: Community is friendly
4. **Open Discussion**: For open-ended questions

### Review Process

**All PRs require:**
1. **Passing CI**: Tests, clippy, fmt
2. **One Approval**: From maintainer
3. **No Unresolved Comments**: Address all feedback

**Timeline:**
- Initial review: 1-3 days
- Follow-up reviews: 1-2 days
- Merge: After approval + CI green

## 🏆 Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` (all contributors)
- Release notes (feature/fix authors)
- Twitter shoutouts (significant contributions)

## 📜 License

By contributing, you agree to license your contributions under:

- **LCL-1.0** (Libertaria Commonwealth License) for core TUI code
- **LSL-1.0** (Libertaria Sovereign License) for plugin SDK code

Both use file-level copyleft – modifications to our files stay open, your larger works stay yours. See [LICENSE](LICENSE) for details.

---

**Questions?** Open an [issue](https://git.sovereign-society.org/ProGit/progit/issues) or join [Discord](https://discord.gg/progit).
