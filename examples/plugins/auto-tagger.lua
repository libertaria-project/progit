-- Example ProGit Plugin: Issue Auto-Tagger
-- SPDX-License-Identifier: Apache-2.0 OR MIT
--
-- Automatically tags issues based on keywords in title/description

local plugin = {}

-- Plugin metadata
plugin.metadata = {
    name = "auto-tagger",
    version = "1.0.0",
    description = "Automatically tag issues based on keywords",
    author = "ProGit Contributors",
    license = "Apache-2.0 OR MIT"
}

-- Configuration
plugin.config = {
    keywords = {
        ["bug"] = "bug",
        ["feature"] = "enhancement",
        ["docs"] = "documentation",
        ["security"] = "security",
        ["performance"] = "performance",
        ["ui"] = "ui",
        ["ux"] = "ux",
        ["api"] = "api",
        ["urgent"] = "priority:high"
    }
}

-- Called when plugin loads
function plugin:on_load()
    print("[auto-tagger] Plugin loaded!")
    return true
end

-- Called when plugin unloads
function plugin:on_unload()
    print("[auto-tagger] Plugin unloading...")
    return true
end

-- Handle events
function plugin:on_event(event)
    if event.type == "IssueCreated" or event.type == "IssueUpdated" then
        return self:analyze_issue(event.data.issue_id)
    end
    return nil
end

-- Analyze issue and suggest tags
function plugin:analyze_issue(issue_id)
    -- In a real plugin, we would call back to ProGit to get issue details
    -- For now, this is a stub showing the intended API
    
    local suggested_tags = {}
    
    -- Example: analyze title for keywords
    -- local issue = progit.get_issue(issue_id)
    -- local text = (issue.title .. " " .. issue.description):lower()
    
    -- for keyword, tag in pairs(self.config.keywords) do
    --     if text:find(keyword) then
    --         table.insert(suggested_tags, tag)
    --     end
    -- end
    
    return {
        action = "suggest_tags",
        issue_id = issue_id,
        tags = suggested_tags
    }
end

-- Execute commands
function plugin:execute_command(command, args)
    if command == "list-keywords" then
        local keywords = {}
        for kw, tag in pairs(self.config.keywords) do
            table.insert(keywords, kw .. " → " .. tag)
        end
        return table.concat(keywords, "\n")
    elseif command == "add-keyword" then
        if #args >= 2 then
            self.config.keywords[args[1]] = args[2]
            return "Added keyword: " .. args[1] .. " → " .. args[2]
        else
            return "Usage: plugin auto-tagger add-keyword <keyword> <tag>"
        end
    else
        return "Unknown command: " .. command
    end
end

return plugin
