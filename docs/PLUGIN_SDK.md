# ProGit Plugin SDK

**License:** Apache-2.0 (allows proprietary plugins)

## Overview

ProGit's plugin system enables extending functionality with Lua scripts while keeping the binary <7MB. Plugins can respond to events, execute commands, and integrate with the workflow.

## Current Integration Contract

Plugins expose host-visible endpoints through `.progit-plugin.json` under `contributions`. ProGit does not read legacy root-level `commands`.

```json
{
  "name": "example-plugin",
  "hooks": ["on_command"],
  "contributions": {
    "commands": [
      {
        "name": "example",
        "title": "Example",
        "description": "Run example commands",
        "entrypoint": "on_command",
        "args": "passthrough",
        "palette": true,
        "tui": {
          "show_output": "modal"
        }
      }
    ]
  }
}
```

The runtime hook receives:

```lua
function on_command(data)
    -- data.command is the contributed namespace
    -- data.args is argv after the namespace
    return {
        handled = true,
        success = true,
        output = "done",
        data = {}
    }
end
```

## Quick Start

### 1. Create a Plugin

```lua
-- my-plugin.lua
local plugin = {}

plugin.metadata = {
    name = "my-plugin",
    version = "1.0.0",
    description = "My awesome plugin",
    author = "Your Name",
    license = "Apache-2.0"
}

function plugin:on_load()
    print("Plugin loaded!")
    return true
end

function plugin:on_event(event)
    if event.type == "IssueCreated" then
        print("New issue: " .. event.data.issue_id)
    end
    return nil
end

function on_command(data)
    if data.command == "hello" then
        return {
            handled = true,
            success = true,
            output = "Hello from my-plugin!"
        }
    end

    return { handled = false }
end

return plugin
```

### 2. Install Plugin

```bash
# Copy plugin into ProGit plugin folder
mkdir -p ~/.progit/plugins/
cp my-plugin.lua ~/.progit/plugins/

# Enable plugin (via TUI or config)
prog config plugin.my-plugin.enabled=true
```

### 3. Use Plugin

```bash
# Trigger command
:plugin hello

# Plugin automatically receives events
```

## Plugin API

### Metadata

Every plugin must export metadata:

```lua
plugin.metadata = {
    name = "plugin-name",     -- Unique identifier
    version = "1.0.0",        -- Semantic version
    description = "...",      -- Short description
    author = "Your Name",     -- Author
    license = "Apache-2.0"    -- License (can be proprietary)
}
```

### Lifecycle Hooks

```lua
function plugin:on_load()
    -- Called when plugin loads
    -- Return true on success, false on error
end

function plugin:on_unload()
    -- Called before plugin unloads
    -- Cleanup resources here
end
```

### Event Handling

```lua
function plugin:on_event(event)
    -- Handle ProGit events
    -- event = { type = "EventType", data = {...} }
    
    -- Return nil or JSON response
    return { action = "...", payload = {...} }
end
```

### Available Events

- `Startup` - ProGit started
- `IssueCreated { issue_id }` - New issue created
- `IssueUpdated { issue_id }` - Issue updated
- `IssueStatusChanged { issue_id, old_status, new_status }` - Status changed
- `Commit Created { commit_hash }` - Commit created
- `BranchCreated { branch_id }` - Virtual branch created
- `BranchUpdated { branch_id }` - Virtual branch updated
- `AgentAction { action, branch_id }` - AI agent action triggered
- `Custom { name, payload }` - Custom plugin-to-plugin event

### Commands

Plugins expose command namespaces in `contributions.commands` and implement `on_command(data)`.

```lua
function on_command(data)
    -- data.command = contributed command namespace
    -- data.args = argv after the namespace

    if data.command ~= "my-command" then
        return { handled = false }
    end

    return {
        handled = true,
        success = true,
        output = "Command output",
        data = {}
    }
end
```

Execute via TUI or CLI:

```text
:plugin my-command [args...]
prog plugin my-command [args...]
```

## Configuration

Plugins can read configuration from `~/.progit/config.kdl`:

```kdl
plugin "my-plugin" {
    enabled true
    config {
        api_key "secret-key"
        endpoint "https://api.example.com"
    }
}
```

Access in plugin:
```lua
-- Configuration passed via plugin:on_load()
function plugin:on_load(config)
    self.api_key = config.api_key or "default"
end
```

## Example Plugins

### 1. Auto-Tagger

Automatically tags issues based on keywords:

```lua
local plugin = {
    metadata = {
        name = "auto-tagger",
        version = "1.0.0"
    },
    keywords = {
        ["bug"] = "bug",
        ["feature"] = "enhancement"
    }
}

function plugin:on_event(event)
    if event.type == "IssueCreated" then
        local tags = {}
        -- Analyze issue and suggest tags
        return { action = "suggest_tags", tags = tags }
    end
end

return plugin
```

### 2. Jira Sync

Sync issues with Jira:

```lua
local plugin = {
    metadata = {
        name = "jira-sync",
        version = "1.0.0"
    }
}

function plugin:on_event(event)
    if event.type == "IssueCreated" then
        -- Call Jira API to create issue
        -- (HTTP client would be provided by SDK)
    end
end

return plugin
```

### 3. Commit Linter

Validate commit messages:

```lua
local plugin = {
    metadata = {
        name = "commit-linter",
        version = "1.0.0"
    }
}

function plugin:on_event(event)
    if event.type == "CommitCreated" then
        local msg = event.data.message
        if not msg:match("^(feat|fix|docs|chore):") then
            return {
                action = "reject",
                reason = "Commit must follow Conventional Commits"
            }
        end
    end
end

return plugin
```

## Best Practices

1. **Keep plugins small** - Do one thing well
2. **Handle errors gracefully** - Return descriptive error messages
3. **Document configuration** - Provide examples
4. **Test thoroughly** - Use ProGit's test mode
5. **License appropriately** - Apache-2.0 or MIT recommended for community plugins

## Publishing Plugins

Share your plugins with the community:

1. Create GitHub repo
2. Add `progit-plugin` topic
3. Tag releases with semantic versioning
4. Submit to ProGit Plugin Registry (coming soon)

## Advanced Features

### HTTP Requests (coming soon)

```lua
local http = require("progit.http")

local response = http.get("https://api.example.com")
if response.status == 200 then
    local json = json.decode(response.body)
end
```

### Database Access (coming soon)

```lua
local db = require("progit.db")

local issues = db.query("SELECT * FROM issues WHERE status = ?", "open")
```

### File System (sandboxed)

```lua
local fs = require("progit.fs")

local content = fs.read_file("~/.progit/data.json")
fs.write_file("~/.progit/output.txt", "content")
```

## Troubleshooting

### Plugin not loading

1. Check syntax: `lua -l my-plugin.lua`
2. Verify metadata is correct
3. Check ProGit logs: `prog --debug`

### Events not firing

1. Ensure plugin is enabled in config
2. Check event type spelling
3. Verify `on_event` function signature

### Command not found

1. Verify the command is declared in `contributions.commands`
2. Check TUI command syntax: `:plugin <command>`

## Support

- [Documentation](https://github.com/progit-io/progit/docs)
- [Examples](https://github.com/progit-io/progit/examples/plugins)
- [Community Plugins](https://github.com/topics/progit-plugin)
- [Discord](https://discord.gg/progit)

## License

Plugin SDK: **Apache-2.0** (allows commercial plugins)  
Core ProGit: **EUPL-1.2** (copyleft)

You can write proprietary plugins - the Apache-2.0 SDK license permits this.
