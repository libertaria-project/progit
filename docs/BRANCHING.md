# Branching Strategy

ProGit uses a three-branch model:

```text
feature/*  ->  forge  ->  main  ->  stable
              dev       reviewed   releases
```

This keeps experimental work away from release artifacts. Do not mix branch
roles; releases must be promoted through `stable`.

## Branches

### `stable` - tagged releases

- **Purpose**: Official release history only.
- **Protection**: No direct feature work.
- **Tags**: Release tags are created on this branch.
- **Source**: Receives reviewed code from `main` during release.
- **Status**: Always buildable and suitable for users.

### `main` - reviewed integration

- **Purpose**: Reviewed, tested code ready for release promotion.
- **Workflow**: Maintainers promote validated work from `forge`.
- **CI/CD**: Quality checks must pass before promotion.
- **Release role**: `./scripts/release.sh` must run from this branch.

### `forge` - active development

- **Purpose**: Contributor and agent landing zone.
- **Workflow**: Feature branches and worktree branches target `forge`.
- **Stability**: Must remain useful, but it can move faster than `main`.

### `feature/*` and `fix/*` - task branches

- **Purpose**: Individual features or bug fixes.
- **Naming**: Use `feature/descriptive-name` or `fix/issue-description`.
- **Lifecycle**: Create from `forge`, merge back to `forge`, then delete.

## Versioning

ProGit follows Semantic Versioning (SemVer) with pre-1.0 conventions:

```text
0.MINOR.PATCH
```

- **PATCH** (`0.2.0` -> `0.2.1`): Bug fixes and small improvements.
- **MINOR** (`0.2.1` -> `0.3.0`): Features and pre-1.0 breaking changes.

### Conventional commits

All commits must follow the
[Conventional Commits](https://www.conventionalcommits.org/) format:

```text
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Common types:

- `feat:` - New feature; bumps MINOR.
- `fix:` - Bug fix; bumps PATCH.
- `docs:` - Documentation only.
- `style:` - Code style or formatting.
- `refactor:` - Code refactoring.
- `test:` - Tests only.
- `chore:` - Maintenance tasks.

Examples:

```bash
feat(tui): add merge request dashboard
fix(sync): prevent duplicate issues on sync
docs: update branching strategy
chore: bump version to 0.8.1
```

## Workflow

### Daily development

1. **Start new work:**

   ```bash
   git checkout forge
   git pull origin forge
   git checkout -b feature/my-feature
   ```

2. **Make changes with conventional commits:**

   ```bash
   git add .
   git commit -m "feat(core): add new feature"
   ```

3. **Run the quality gate:**

   ```bash
   ./scripts/check.sh
   ```

4. **Push and open a merge request to `forge`:**

   ```bash
   git push origin feature/my-feature
   ```

Maintainers promote `forge` to `main` after review and validation.

### Creating a release

1. **Switch to `main`:**

   ```bash
   git checkout main
   git pull origin main
   ```

2. **Bump version:**

   ```bash
   ./scripts/bump-version.sh
   ```

3. **Run the release workflow:**

   ```bash
   ./scripts/release.sh
   ```

   This will:

   - Run quality checks.
   - Build the release binary.
   - Merge `main` -> `stable`.
   - Create tag `vX.Y.Z` on `stable`.
   - Push `main`, `stable`, and the tag.
   - Return to `main`.

4. **Post-release tasks:**

   - Create Forgejo release notes.
   - Build and publish binaries if needed.
   - `scripts/release.sh` now updates the `progit-bin` AUR package automatically.
     Set `SKIP_AUR_UPDATE=true` if your environment is not AUR-capable.

### Dry run

Test scripts without changing branches or writing tags:

```bash
./scripts/bump-version.sh --dry-run
./scripts/release.sh --dry-run
```

`release.sh --dry-run` may run with uncommitted changes so you can test script
edits. Real releases require a clean working tree.

## Quick reference

| Action | Command |
|--------|---------|
| Create feature branch | `git checkout -b feature/name forge` |
| Push feature branch | `git push origin feature/name` |
| Quality gate | `./scripts/check.sh` |
| Bump version | `./scripts/bump-version.sh` |
| Release | `./scripts/release.sh` |
| Check current version | `grep '^version' Cargo.toml` |
| List tags | `git tag -l` |

## Branch protection

For Forgejo, GitHub, or GitLab, configure:

- **`stable` branch:**
  - Require maintainer-controlled releases.
  - Require status checks to pass.
  - Block direct feature merges.

- **`main` branch:**
  - Require merge request reviews.
  - Require status checks to pass.
  - Require branches to be up to date.
  - No unreviewed direct pushes.

- **`forge` branch:**
  - Require status checks to pass.
  - Accept reviewed feature and agent work.

## Migration from older branch models

If the repository only has `main` and `stable`, create `forge` from `main`:

```bash
git checkout main
git pull origin main
git checkout -b forge
git push -u origin forge
```

If the repository still has `develop`, stop using it for new work. Promote any
remaining reviewed commits into `forge` or `main`, then retire `develop` after
maintainer approval.
