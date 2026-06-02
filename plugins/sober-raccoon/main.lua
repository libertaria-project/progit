-- SPDX-License-Identifier: LicenseRef-ProGit-Premium
-- ProGit Premium Plugin: sober-raccoon
-- Sober governance cockpit powered by the host Sober bridge.

plugin = {
    name = "sober-raccoon",
    version = "0.1.0",
    author = "ProGit Team",
    description = "Premium Sober governance cockpit for ProGit",
    sdk_api = "0.3",
    hooks = {}
}

local config = {
    base = "HEAD",
    provider = "kimi-coding",
    model = "kimi-k2.6",
    reviewer = "security",
    objective = "security",
    hygiene_profile = "standard",
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

local function preflight(payload)
    local base = payload.base or config.base
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "preflight",
        base = base,
        result = call_sober("preflight", { base = base }),
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
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "review-preview",
        result = call_sober("review-preview", opts),
    }
end

local function hygiene(payload)
    local profile = payload.profile or config.hygiene_profile
    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "hygiene",
        profile = profile,
        result = call_sober("hygiene", { profile = profile }),
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

    return {
        plugin = "sober-raccoon",
        premium = true,
        action = "hooks",
        result = call_sober(action, { hook = payload.hook }),
    }
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

    if action == "status" then
        return status(payload)
    elseif action == "preflight" then
        return preflight(payload)
    elseif action == "review-preview" then
        return review_preview(payload)
    elseif action == "hygiene" then
        return hygiene(payload)
    elseif action == "hooks" then
        return hooks(payload)
    end

    return {
        plugin = "sober-raccoon",
        premium = true,
        action = action,
        ok = false,
        error = "Unsupported sober-raccoon action: " .. tostring(action),
    }
end
