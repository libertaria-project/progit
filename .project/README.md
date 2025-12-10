# .project Directory Structure

This directory contains your project's issue tracking data.

## Contents

- **`issues/`** - Issue files in KDL format (committed to git)
- **`config.kdl`** - User-specific configuration (gitignored, auto-generated on first run)

## First Time Setup

When you run `prog` for the first time, it will:

1. Detect your git remote (if any)
2. Generate `.project/config.kdl` with appropriate sync settings
3. Load existing issues from `issues/*.kdl`

If you want to customize the config, copy `config.kdl.example` from the project root:

```bash
cp config.kdl.example .project/config.kdl
# Edit .project/config.kdl with your settings
```

## Issue Storage

Issues are stored as human-readable KDL files in `issues/`. Each issue is a separate file, making it easy to:

- Track changes in git
- Review issues in pull requests
- Search/grep through issues
- Sync with remote forges (GitLab, Forgejo, GitHub)

The filename format is: `{id}-{slug}.kdl`

Example: `example-001-welcome.kdl`
