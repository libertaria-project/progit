-- SPDX-License-Identifier: LicenseRef-ProGit-Premium
-- ProGit Premium Plugin: sober-raccoon
-- Sober governance cockpit powered by the host Sober bridge.

plugin = {
    name = "sober-raccoon",
    version = "0.1.0",
    author = "ProGit Team",
    description = "Premium Sober governance cockpit for ProGit",
    sdk_api = "0.3",
    hooks = {
        on_command = true,
    },
    commands = { "sober", "sober-raccoon" },
}

local config = {
    base = "HEAD",
    provider = "kimi-coding",
    model = "kimi-k2.6",
    reviewer = "security",
    objective = "security",
    hygiene_profile = "standard",
}

local routes = {
    { name = "prog plugin sober <args...>", description = "Run any Sober CLI command through the host bridge" },
    { name = "prog plugin sober route list", description = "Run Sober route list" },
    { name = "prog plugin sober report list", description = "Run Sober report list" },
    { name = "prog plugin sober assist readiness --target release", description = "Run Sober assist readiness" },
    { name = "prog plugin sober-raccoon status", description = "Run doctor, preflight, and hooks status summary" },
    { name = "prog plugin sober-raccoon route list", description = "List Sober Raccoon routes" },
    { name = "prog plugin sober-raccoon help", description = "Show command usage" },
}

local function merge_config(source)
    if not source then
        return
    end
    for key, value in pairs(source) do
        if config[key] ~= nil and value ~= nil then
            config[key] = value
        end
    end
end

function init()
    if context and context.config then
        merge_config(context.config)
    end
    log.info("sober-raccoon premium governance plugin initialised")
end

local function call_sober(action, opts)
    if not sober or not sober.run then
        return {
            ok = false,
            error = "Sober host capability is unavailable",
        }
    end

    local ok, result = pcall(function()
        return sober.run(action, opts or {})
    end)
    if not ok then
        return {
            ok = false,
            error = tostring(result),
        }
    end
    return result
end

local function status(payload)
    local base = payload.base or config.base
    local doctor = call_sober("doctor", {})
    local preflight = call_sober("preflight", { base = base })
    local hooks = call_sober("hooks-status", {})

    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "status",
        ok = doctor.ok and preflight.ok and hooks.ok,
        base = base,
        doctor = doctor,
        preflight = preflight,
        hooks = hooks,
    }
end

local function doctor(payload)
    local result = call_sober("doctor", { online = payload.online })
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "doctor",
        ok = result.ok,
        result = result,
    }
end

local function preflight(payload)
    local base = payload.base or config.base
    local result = call_sober("preflight", { base = base })
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "preflight",
        ok = result.ok,
        base = base,
        result = result,
    }
end

local function review_preview(payload)
    local opts = {
        base = payload.base or config.base,
        provider = payload.provider or config.provider,
        model = payload.model or config.model,
        reviewer = payload.reviewer or config.reviewer,
        objective = payload.objective or config.objective,
    }
    local result = call_sober("review-preview", opts)
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "review-preview",
        ok = result.ok,
        result = result,
    }
end

local function hygiene(payload)
    local profile = payload.profile or config.hygiene_profile
    local result = call_sober("hygiene", { profile = profile })
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "hygiene",
        ok = result.ok,
        profile = profile,
        result = result,
    }
end

local function hooks(payload)
    local method = payload.method or "status"
    local action = "hooks-status"
    if method == "install" then
        action = "hooks-install"
    elseif method ~= "status" then
        return {
            plugin = "sober-raccoon",
            premium = true,
            action = "hooks",
            ok = false,
            error = "Unsupported hooks method: " .. tostring(method),
        }
    end

    local result = call_sober(action, { hook = payload.hook })
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "hooks",
        ok = result.ok,
        method = method,
        hook = payload.hook,
        result = result,
    }
end

local function cli(args)
    local result = call_sober("cli", { args = args or {} })
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "cli",
        ok = result.ok,
        command = args or {},
        result = result,
    }
end

local function route(payload)
    local method = payload.method or "list"
    if method ~= "list" then
        return {
            plugin = "sober-raccoon",
            premium = true,
            action = "route",
            ok = false,
            error = "Unsupported route command: " .. tostring(method) .. ". Use: route list",
        }
    end

    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "route list",
        ok = true,
        routes = routes,
    }
end

local function action_response(action, payload)
    if action == "status" then
        return status(payload)
    elseif action == "doctor" then
        return doctor(payload)
    elseif action == "preflight" then
        return preflight(payload)
    elseif action == "review-preview" then
        return review_preview(payload)
    elseif action == "hygiene" then
        return hygiene(payload)
    elseif action == "hooks" then
        return hooks(payload)
    elseif action == "route" or action == "routes" or action == "list" then
        return route(payload)
    end

    return cli(payload.args or {})
end

local function has_flag(args, flag)
    for _, value in ipairs(args or {}) do
        if value == flag then
            return true
        end
    end
    return false
end

local function value_after(args, flag, default)
    for index, value in ipairs(args or {}) do
        if value == flag and args[index + 1] then
            return args[index + 1]
        end
    end
    return default
end

local function command_payload(args)
    local action = args[1] or "status"
    if string.sub(action, 1, 1) == "-" then
        action = "status"
    end
    local payload = {
        base = value_after(args, "--base", config.base),
        provider = value_after(args, "--provider", config.provider),
        model = value_after(args, "--model", config.model),
        reviewer = value_after(args, "--reviewer", config.reviewer),
        objective = value_after(args, "--objective", config.objective),
        profile = value_after(args, "--profile", config.hygiene_profile),
        online = has_flag(args, "--online"),
    }

    if action == "hooks" then
        payload.method = args[2] or "status"
        payload.hook = args[3]
    elseif action == "route" then
        payload.method = args[2] or "list"
    end
    payload.args = args

    return action, payload
end

local function response_ok(response)
    if response.ok ~= nil then
        return response.ok
    end
    if response.result and response.result.ok ~= nil then
        return response.result.ok
    end
    return false
end

local function response_error(response)
    if type(response.error) == "string" and response.error ~= "" then
        return response.error
    end
    if response.result and type(response.result.error) == "string" and response.result.error ~= "" then
        return response.result.error
    end
    if response.doctor and type(response.doctor.error) == "string" and response.doctor.error ~= "" then
        return response.doctor.error
    end
    if response.preflight and type(response.preflight.error) == "string" and response.preflight.error ~= "" then
        return response.preflight.error
    end
    if response.hooks and type(response.hooks.error) == "string" and response.hooks.error ~= "" then
        return response.hooks.error
    end
    return nil
end

local function command_error(response)
    if response.action == "cli" and not response_ok(response) then
        local data = response.result and response.result.data or {}
        if type(data.status) == "string" and data.status ~= "" then
            return "sober exited with " .. data.status
        end
        return "sober command failed"
    end

    return response_error(response)
end

local function usage()
    return table.concat({
        "Usage:",
        "  prog plugin sober <any sober subcommand> [args...]",
        "  prog plugin sober-raccoon <status|route list|help>",
        "",
        "Examples:",
        "  prog plugin sober preflight --base HEAD",
        "  prog plugin sober route list",
        "  prog plugin sober report list",
        "  prog plugin sober assist readiness --target release",
        "  prog plugin sober --version",
        "  prog plugin sober-raccoon status",
    }, "\n")
end

local function trim_trailing_newlines(value)
    if type(value) ~= "string" then
        return ""
    end
    return string.gsub(value, "[\r\n]+$", "")
end

local function command_output(response)
    if response.routes then
        local lines = { "Sober Raccoon routes:" }
        for _, route_entry in ipairs(response.routes) do
            table.insert(lines, "  " .. route_entry.name .. " - " .. route_entry.description)
        end
        return table.concat(lines, "\n")
    end

    if response.action == "cli" then
        local data = response.result and response.result.data or {}
        local stdout = trim_trailing_newlines(data.stdout)
        local stderr = trim_trailing_newlines(data.stderr)
        local lines = {}
        if stdout ~= "" then
            table.insert(lines, stdout)
        end
        if stderr ~= "" then
            if #lines > 0 then
                table.insert(lines, "stderr:")
            end
            table.insert(lines, stderr)
        end
        if #lines == 0 then
            table.insert(lines, "Sober command: " .. (response_ok(response) and "OK" or "FAIL"))
        end
        return table.concat(lines, "\n")
    end

    local ok = response_ok(response)
    local lines = {
        "Sober Raccoon " .. tostring(response.action or "command") .. ": " .. (ok and "OK" or "FAIL"),
    }
    if response.base then
        table.insert(lines, "base: " .. tostring(response.base))
    end
    if response.profile then
        table.insert(lines, "profile: " .. tostring(response.profile))
    end
    if response.method then
        table.insert(lines, "hooks: " .. tostring(response.method))
    end
    local err = response_error(response)
    if err then
        table.insert(lines, "error: " .. tostring(err))
    end
    return table.concat(lines, "\n")
end

local function cli_response(response, json_mode)
    return {
        handled = true,
        success = response_ok(response),
        output = json_mode and json.encode(response) or command_output(response),
        error = command_error(response),
        data = response,
    }
end

function on_command(data)
    local command = data.command
    if command ~= "sober" and command ~= "sober-raccoon" then
        return { handled = false, success = false }
    end

    local args = data.args or {}
    if args[1] == "help" then
        return {
            handled = true,
            success = true,
            output = usage(),
        }
    end

    if command == "sober" then
        return cli_response(cli(args), false)
    end

    if has_flag(args, "--help") or has_flag(args, "-h") then
        return {
            handled = true,
            success = true,
            output = usage(),
        }
    end

    local action, payload = command_payload(args)
    local response = action_response(action, payload)
    return cli_response(response, has_flag(args, "--json"))
end

function plugin.on_event(event)
    if not event or event.type ~= "Custom" then
        return nil
    end

    local data = event.data or {}
    if data.name ~= "sober-raccoon" and data.name ~= "sober-raccoon.query" then
        return nil
    end

    local payload = data.payload or {}
    local action = payload.action or "status"
    return action_response(action, payload)
end
