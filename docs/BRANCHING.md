# Branching Strategy

ProGit uses a **two-branch model** for pragmatic version control:

```
feature/xyz → develop (active development)
              ↓
         (when stable)
              ↓
         develop → main (tagged releases)
```

## Branches

### `main` - Stable Releases
- **Purpose**: Production-ready code only
- **Protection**: No direct commits
- **Tags**: Every merge is tagged with semantic version (e.g., `v0.3.0`)
- **Status**: Always buildable and functional

### `develop` - Active Development
- **Purpose**: Integration branch for all features and fixes
- **Workflow**: All work lands here first via feature branches
- **CI/CD**: Automated tests run on every push
- **Stability**: Should be stable, but may have minor issues

### `feature/*` - Feature Branches
- **Purpose**: Individual features or bug fixes
- **Naming**: `feature/descriptive-name` or `fix/issue-description`
- **Lifecycle**: Created from `develop`, merged back to `develop`
- **Cleanup**: Delete after merge

## Versioning

ProGit follows **Semantic Versioning** (SemVer) with pre-1.0 conventions:

```
0.MINOR.PATCH
```

- **PATCH** (`0.2.0` → `0.2.1`): Bug fixes, small improvements
  - Triggered by: `fix:` commits
  
- **MINOR** (`0.2.1` → `0.3.0`): New features, breaking changes OK before 1.0
  - Triggered by: `feat:` commits or `BREAKING CHANGE:` footer

### Conventional Commits

All commits should follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat:` - New feature (bumps MINOR)
- `fix:` - Bug fix (bumps PATCH)
- `docs:` - Documentation only
- `style:` - Code style/formatting
- `refactor:` - Code refactoring
- `test:` - Adding tests
- `chore:` - Maintenance tasks

**Examples:**
```bash
feat(tui): add merge request dashboard
fix(sync): prevent duplicate issues on sync
docs: update branching strategy
chore: bump version to 0.3.0
```

## Workflow

### Daily Development

1. **Start new work:**
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/my-feature
   ```

2. **Make changes with conventional commits:**
   ```bash
   git add .
   git commit -m "feat(core): add new feature"
   ```

3. **Merge to develop:**
   ```bash
   git checkout develop
   git merge feature/my-feature --no-ff
   git push origin develop
   git branch -d feature/my-feature
   ```

### Creating a Release

1. **Bump version (analyzes commits automatically):**
   ```bash
   ./scripts/bump-version.sh
   ```

2. **Run release workflow:**
   ```bash
   ./scripts/release.sh
   ```

   This will:
   - Run tests
   - Merge `develop` → `main`
   - Create git tag
   - Push to remote
   - Return to `develop`

3. **Post-release tasks:**
   - Create GitHub/GitLab release notes
   - Build and publish binaries
   - Update AUR package

### Dry Run (Testing)

Test scripts without making changes:

```bash
./scripts/bump-version.sh --dry-run
./scripts/release.sh --dry-run
```

## Quick Reference

| Action | Command |
|--------|---------|
| Create feature branch | `git checkout -b feature/name` |
| Merge to develop | `git checkout develop && git merge feature/name --no-ff` |
| Bump version | `./scripts/bump-version.sh` |
| Release | `./scripts/release.sh` |
| Check current version | `grep '^version' Cargo.toml` |
| List tags | `git tag -l` |

## Branch Protection (Recommended)

For GitHub/GitLab, configure:

- **`main` branch:**
  - Require pull request reviews
  - Require status checks to pass
  - Require branches to be up to date
  - No direct pushes

- **`develop` branch:**
  - Require status checks to pass
  - Allow direct pushes (for single developer)

## Migration from Current State

If you're currently on `main` with no `develop` branch:

```bash
# Create develop from current main
git checkout -b develop
git push -u origin develop

# Continue working on develop
git checkout develop
```

All future work happens on `develop`. Use `main` only for releases.
