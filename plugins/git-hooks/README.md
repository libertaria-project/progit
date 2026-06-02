# ProGit Git Hooks Plugin

Smart git hooks management for ProGit — validated, configurable, and team-friendly.

## Features

- **Commit Message Validation** — Enforce Conventional Commits, Angular, or custom formats
- **Branch Naming Enforcement** — Configurable naming patterns (e.g., `type/issue-description`)
- **Pre-commit Checks** — Run linting, formatting, tests before commit
- **Per-repo Configuration** — Each project can override rules via `PANOPTICUM.kdl`
- **Easy Installation** — Single command to install all hooks

## Installation

```bash
# Install via ProGit CLI
prog plugin install git-hooks

# Install hooks to current repository
prog hooks install
```

## Configuration

Add to your `PANOPTICUM.kdl`:

```kdl
hooks {
    enabled true
    commit-msg {
        style "conventional"  // conventional | angular | custom
        allow-empty false
    }
    branch {
        pattern "^(feat|fix|chore|docs|style|refactor|test|perf|ci)/[a-z0-9-]+$"
        require-issue false
    }
    pre-commit {
        run-linters true
        run-formatters false
        fail-on-warning false
    }
}
```

## Commands

| Command | Description |
|---------|-------------|
| `prog hooks install` | Install all configured hooks |
| `prog hooks uninstall` | Remove all installed hooks |
| `prog hooks status` | Show which hooks are installed |
| `prog hooks validate <type>` | Run validation manually |
| `prog hooks list` | List available hook types |

## Hook Types

### commit-msg

Validates commit messages before commit is recorded.

**Conventional Commits format:**
```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `ci`, `chore`, `revert`

### branch

Validates branch names against patterns.

**Default pattern:** `^(feat|fix|chore|docs|style|refactor|test|perf|ci)/[a-z0-9-]+$`

Examples:
- `feat/add-login-form` ✅
- `fix/issue-123` ✅
- `my-branch` ❌

### pre-commit

Runs before each commit (requires configured tools).

**Checks:**
- File size limits
- Secret detection
- Linter checks (if configured)
- Merge conflict markers

## License

Apache-2.0
