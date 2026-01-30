-- GitLab CI/CD Pipeline Status Plugin
-- Queries GitLab API for MR pipeline status

plugin = {
    name = "gitlab-ci",
    version = "1.0.0",
    author = "ProGit Team",
    description = "Queries GitLab CI/CD pipeline status for merge requests",
    hooks = {}
}

-- Configuration (loaded from .progit-plugin.json or user config)
local config = {
    api_url = nil,      -- e.g. "https://gitlab.com/api/v4"
    private_token = nil, -- GitLab Personal Access Token
}

-- Simple HTTP GET implementation using os.execute and curl
-- (In production, this would use a proper HTTP library)
local function http_get(url, headers)
    local cmd = string.format('curl -s')
    
    for key, value in pairs(headers or {}) do
        cmd = cmd .. string.format(" -H '%s: %s'", key, value)
    end
    
    cmd = cmd .. string.format(" '%s'", url)
    
    local handle = io.popen(cmd)
    if not handle then
        return nil, "Failed to execute curl"
    end
    
    local result = handle:read("*a")
    handle:close()
    
    return result, nil
end

-- Parse JSON response (basic implementation)
local function parse_json(str)
    -- This is a simplified parser - in production use a proper JSON library
    -- For now, we'll use Lua's load() with a safe environment
    local json_str = str:gsub('null', 'nil'):gsub('true', 'true'):gsub('false', 'false')
    
    -- Attempt to decode using Lua's built-in functions
    local ok, result = pcall(function()
        return load("return " .. json_str)()
    end)
    
    if ok then
        return result
    else
        return nil
    end
end

-- Map GitLab pipeline status to ProGit status
local function parse_pipeline_status(gitlab_status)
    local status_map = {
        success = "passed",
        failed = "failed",
        running = "running",
        pending = "pending",
        canceled = "canceled",
        skipped = "skipped",
        manual = "pending",
    }
    
    return status_map[gitlab_status] or "unknown"
end

-- Initialize plugin with context
function init(ctx)
    -- Extract config from context
    if ctx and ctx.config then
        config.api_url = ctx.config.gitlab_api_url or ctx.config.api_url
        config.private_token = ctx.config.gitlab_token or ctx.config.private_token
    end
    
    -- Validate configuration
    if not config.api_url then
        error("gitlab-ci plugin requires 'gitlab_api_url' in config")
    end
    
    if not config.private_token then
        error("gitlab-ci plugin requires 'gitlab_token' in config")
    end
    
    print("gitlab-ci plugin initialized")
end

-- Handle plugin events (new event-based API)
function plugin.on_event(event)
    -- Only handle PipelineStatusQuery events
    if event.type ~= "PipelineStatusQuery" then
        return nil
    end
    
    local data = event.data or event
    
    -- Validate this is a GitLab query
    if data.forge_type ~= "gitlab" then
        return nil
    end
    
    -- Build GitLab API URL
    -- GET /projects/:id/merge_requests/:merge_request_iid/pipelines
    local api_url = string.format(
        "%s/projects/%s/merge_requests/%s/pipelines",
        data.api_url or config.api_url,
        data.project_id,
        data.mr_id
    )
    
    -- Query GitLab API
    local response, err = http_get(api_url, {
        ["PRIVATE-TOKEN"] = config.private_token
    })
    
    if err then
        print("gitlab-ci: Failed to query API: " .. err)
        return {
            status = "unknown",
            error = err
        }
    end
    
    -- Parse JSON response
    local pipelines = parse_json(response)
    
    if not pipelines or type(pipelines) ~= "table" or #pipelines == 0 then
        -- No pipelines found
        return {
            status = "unknown"
        }
    end
    
    -- Get the latest pipeline (first in array)
    local pipeline = pipelines[1]
    
    -- Extract job information (if available)
    local jobs = {}
    if pipeline.jobs then
        for _, job in ipairs(pipeline.jobs) do
            table.insert(jobs, {
                name = job.name,
                status = parse_pipeline_status(job.status),
                stage = job.stage,
            })
        end
    end
    
    -- Return pipeline status
    return {
        status = parse_pipeline_status(pipeline.status),
        pipeline_id = tostring(pipeline.id),
        jobs = jobs,
        updated_at = pipeline.updated_at,
        web_url = pipeline.web_url,
    }
end
