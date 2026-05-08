-- SPDX-License-Identifier: Apache-2.0
-- Example ProGit Plugin: Issue Logger
-- Logs all issue events using the storage API for persistence

plugin = {
    name = "issue-logger",
    version = "1.0.0",
    author = "ProGit Team",
    description = "Logs all issue events using storage API",
    hooks = {
        on_issue_created = true,
        on_issue_updated = true,
        on_issue_deleted = true,
    }
}

function init()
    -- Initialize storage - logs are stored via storage API
    log_info("issue-logger plugin initialized (using storage API)")
end

function log_event(event_type, issue)
    local entry = string.format(
        "[%s] %s: %s (ID: %s, Status: %s)",
        os.date("%Y-%m-%d %H:%M:%S"),
        event_type,
        issue.title,
        issue.id,
        issue.status or "unknown"
    )
    
    -- Get existing log entries
    local log_key = "issue_events"
    local ok, events = pcall(function()
        return storage.get(log_key) or {}
    end)
    
    if not ok or not events then
        events = {}
    end
    
    -- Add new entry
    table.insert(events, entry)
    
    -- Store updated log
    ok, err = pcall(function()
        storage.set(log_key, events)
    end)
    
    if not ok then
        log_error("Failed to log event: " .. tostring(err))
    end
    
    return { success = true }
end

function on_issue_created(issue)
    return log_event("CREATED", issue)
end

function on_issue_updated(issue)
    return log_event("UPDATED", issue)
end

function on_issue_deleted(issue)
    return log_event("DELETED", issue)
end
