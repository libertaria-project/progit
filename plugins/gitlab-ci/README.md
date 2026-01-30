# GitLab CI/CD Plugin

Displays real-time CI/CD pipeline status for GitLab merge requests directly in the ProGit TUI.

## Features

- ✅ Query pipeline status for all merge requests
- ✅ Display status icons in MR Dashboard (✓ passed, ✗ failed, ● running)
- ✅ Color-coded status indicators
- ✅ Support for all GitLab pipeline states
- ✅ Minimal API calls (cached per refresh)

## Configuration

Add to your `.project/config.kdl`:

```kdl
plugins {
    gitlab-ci {
        gitlab_api_url "https://gitlab.com/api/v4"
        gitlab_token "glpat-xxxxxxxxxxxxxxxxxxxx"
    }
}
```

### Required Configuration

| Key | Description | Example |
|-----|-------------|---------|
| `gitlab_api_url` | GitLab API base URL | `https://gitlab.com/api/v4` |
| `gitlab_token` | Personal Access Token with `read_api` scope | `glpat-xyz123...` |

### Creating a GitLab Token

1. Go to GitLab → Settings → Access Tokens
2. Create token with `read_api` scope
3. Copy token to config (store securely!)

## Usage

The plugin runs automatically when viewing the MR Dashboard (`ViewMode::MRList`).

**Status Icons:**
- `✓` - All jobs passed (green)
- `✗` - One or more jobs failed (red)
- `●` - Pipeline running (yellow)
- `○` - Pipeline pending (dim)
- `⊘` - Pipeline canceled (gray)
- `⊗` - Pipeline skipped (gray)
- `–` - No pipeline or unknown status

## API Details

**Endpoint:** `GET /projects/:id/merge_requests/:mr_iid/pipelines`

**Response:** Array of pipelines (sorted newest first)

**Rate Limiting:** Plugin caches results per MR refresh cycle

## Troubleshooting

**No CI status showing:**
- Verify `gitlab_token` has `read_api` scope
- Check API URL matches your GitLab instance
- Ensure MR has a pipeline (check GitLab web UI)

**"Unknown" status:**
- Pipeline may not exist yet
- Token may be expired or revoked
- API URL may be incorrect

**Plugin not loading:**
- Check logs: `RUST_LOG=debug prog`
- Verify `.progit-plugin.json` syntax
- Ensure `main.lua` exists in plugin directory

## License

Apache-2.0 (same as progit-plugin-sdk)

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for plugin development guidelines.
