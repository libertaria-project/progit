-- SPDX-License-Identifier: Apache-2.0
-- Example ProGit Plugin: Issue Logger
-- Logs all issue events to a file for auditing

plugin = {
    name = "issue-logger",
    version = "1.0.0",
    author = "ProGit Team",
    description = "Logs all issue events to .progit/issue-log.txt",
    hooks = {
        on_issue_created = true,
        on_issue_updated = true,
        on_issue_deleted = true,
    }
}

local log_file = nil

function init()
    -- Open log file in append mode
    local log_path = context.repo_path .. "/.progit/issue-log.txt"
    log_file = io.open(log_path, "a")
    if log_file then
        log_file:write("\n=== Plugin initialized at " .. os.date() .. " ===\n")
        log_file:flush()
    end
end

function on_issue_created(issue)
    if log_file then
        log_file:write(string.format(
            "[%s] CREATED: %s (ID: %s, Status: %s)\n",
            os.date("%Y-%m-%d %H:%M:%S"),
            issue.title,
            issue.id,
            issue.status
        ))
        log_file:flush()
    end
    return { success = true }
end

function on_issue_updated(issue)
    if log_file then
        log_file:write(string.format(
            "[%s] UPDATED: %s (ID: %s, Status: %s)\n",
            os.date("%Y-%m-%d %H:%M:%S"),
            issue.title,
            issue.id,
            issue.status
        ))
        log_file:flush()
    end
    return { success = true }
end

function on_issue_deleted(data)
    if log_file then
        log_file:write(string.format(
            "[%s] DELETED: Issue ID %s\n",
            os.date("%Y-%m-%d %H:%M:%S"),
            data.id
        ))
        log_file:flush()
    end
    return { success = true }
end
