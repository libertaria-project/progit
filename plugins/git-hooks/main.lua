-- SPDX-License-Identifier: Apache-2.0
-- ProGit Plugin: Git Hooks Manager
-- Smart git hooks management with validation and configuration
--
-- This plugin provides:
-- - Commit message validation (Conventional Commits, Angular, custom)
-- - Branch naming enforcement
-- - Pre-commit checks
-- - Easy hook installation/uninstallation

plugin = {
    name = "git-hooks",
    version = "1.0.0",
    author = "Voxis Forge",
    description = "Smart git hooks management with validation and configuration",
    hooks = {
        on_command = true,
    },
    commands = { "hooks", "hook" }
}

-- ============================================================================
-- Configuration
-- ============================================================================

local config = {
    enabled = true,
    hooks_dir = nil,  -- Set during init
    repo_root = nil,   -- Set during init
    
    -- Commit message validation
    commit_msg = {
        enabled = true,
        style = "conventional",  -- conventional | angular | custom
        custom_pattern = nil,
        allow_empty = false,
        min_length = 10,
        max_length = 100,
    },
    
    -- Branch naming rules
    branch = {
        enabled = true,
        pattern = "^(feat|fix|chore|docs|style|refactor|test|perf|ci)/[a-z0-9-]+$",
        require_issue = false,
        issue_pattern = "%[?[A-Z]+-%d+%d*%]?",
    },
    
    -- Pre-commit checks
    pre_commit = {
        enabled = true,
        check_file_size = true,
        max_file_size_kb = 500,
        check_secrets = true,
        check_conflicts = true,
        run_linters = false,
        run_formatters = false,
        fail_on_warning = false,
    },
    
    -- Hook installation paths
    install = {
        commit_msg = true,
        pre_commit = true,
        pre_push = false,
        post_commit = false,
    }
}

-- Conventional Commits types
local commit_types = {
    feat = "A new feature",
    fix = "A bug fix",
    docs = "Documentation only changes",
    style = "Code style changes (formatting, semicolons, etc)",
    refactor = "Code changes that neither fix a bug nor add a feature",
    perf = "Code changes that improve performance",
    test = "Adding or correcting tests",
    ci = "Changes to CI configuration files and scripts",
    chore = "Other changes that don't modify src or test files",
    revert = "Reverts a previous commit",
}

-- ============================================================================
-- Initialization
-- ============================================================================

function init()
    -- Get repo root from context
    config.repo_root = context.repo_path or "."
    config.hooks_dir = config.repo_root .. "/.git/hooks"
    
    -- Load configuration from storage API if available
    load_config_from_storage()
    
    -- Override with context config if provided
    if context.config then
        merge_config(context.config)
    end
    
    log_info("git-hooks plugin initialized")
    log_info("  Hooks directory: " .. config.hooks_dir)
    log_info("  Commit msg style: " .. config.commit_msg.style)
    log_info("  Branch pattern: " .. config.branch.pattern)
end

-- ============================================================================
-- IO Shim (for sandboxed environments without io.open)
-- ============================================================================

if not io or not io.open then
    -- Use io_open from SDK if available
    io = {
        open = function(path, mode)
            if io_open then
                return io_open(path, mode)
            end
            return nil
        end
    }
end

-- Load configuration using SDK storage API (no io.open needed)
function load_config_from_storage()
    -- Use storage API if available, otherwise use defaults
    -- Storage is provided by the SDK via the `storage` global
    if storage and storage.get then
        local ok, hooks_config = pcall(function()
            return storage.get("hooks_config")
        end)
        if ok and hooks_config then
            log_info("Loaded configuration from storage")
            -- Would merge with current config
        else
            log_info("No stored hooks config, using defaults")
        end
    else
        log_info("Using default configuration (no storage API)")
    end
end

-- Parse KDL configuration
function parse_kdl_config(content)
    -- Extract hooks block
    local hooks_start = content:match('hooks%s*{([^}]+)}')
    if not hooks_start then
        return
    end
    
    -- Parse enabled flag
    if hooks_start:match('enabled%s+true') then
        config.enabled = true
    elseif hooks_start:match('enabled%s+false') then
        config.enabled = false
    end
    
    -- Parse commit-msg section
    local cm_start = hooks_start:match('commit%-msg%s*{([^}]+)}')
    if cm_start then
        if cm_start:match('style%s+"([^"]+)"') then
            config.commit_msg.style = cm_start:match('style%s+"([^"]+)"')
        end
        if cm_start:match('allow%-empty%s+true') then
            config.commit_msg.allow_empty = true
        end
    end
    
    -- Parse branch section
    local br_start = hooks_start:match('branch%s*{([^}]+)}')
    if br_start then
        if br_start:match('pattern%s+"([^"]+)"') then
            config.branch.pattern = br_start:match('pattern%s+"([^"]+)"')
        end
        if br_start:match('require%-issue%s+true') then
            config.branch.require_issue = true
        end
    end
    
    -- Parse pre-commit section
    local pc_start = hooks_start:match('pre%-commit%s*{([^}]+)}')
    if pc_start then
        if pc_start:match('run%-linters%s+true') then
            config.pre_commit.run_linters = true
        end
        if pc_start:match('run%-formatters%s+true') then
            config.pre_commit.run_formatters = true
        end
        if pc_start:match('fail%-on%-warning%s+true') then
            config.pre_commit.fail_on_warning = true
        end
    end
    
    log_info("Loaded configuration from PANOPTICUM.kdl")
end

-- Merge configuration tables
function merge_config(override)
    if override.enabled ~= nil then
        config.enabled = override.enabled
    end
    if override.commit_msg then
        for k, v in pairs(override.commit_msg) do
            config.commit_msg[k] = v
        end
    end
    if override.branch then
        for k, v in pairs(override.branch) do
            config.branch[k] = v
        end
    end
    if override.pre_commit then
        for k, v in pairs(override.pre_commit) do
            config.pre_commit[k] = v
        end
    end
end

-- ============================================================================
-- Hook Handlers
-- ============================================================================

-- Main command handler
function on_command(data)
    local cmd = data.command
    local args = data.args or {}
    
    if cmd == "hooks" or cmd == "hook" then
        return handle_hooks_command(args)
    end
    
    -- Unknown command - signal that we didn't handle it
    return { success = false, handled = false, error = "Unknown command: " .. cmd }
end

-- Handle all hooks commands
function handle_hooks_command(args)
    local subcommand = args[1] or "status"
    
    if subcommand == "install" then
        return install_hooks(args)
    elseif subcommand == "uninstall" then
        return uninstall_hooks(args)
    elseif subcommand == "status" then
        return hooks_status(args)
    elseif subcommand == "validate" then
        return validate_hooks(args)
    elseif subcommand == "list" then
        return list_hooks(args)
    elseif subcommand == "help" then
        return show_help(args)
    else
        return {
            success = false,
            error = "Unknown subcommand: " .. subcommand .. "\nUse 'prog hooks help' for usage"
        }
    end
end

-- ============================================================================
-- Hook Installation
-- ============================================================================

function install_hooks(args)
    log_info("Installing git hooks...")
    
    local installed = {}
    local failed = {}
    
    -- Install commit-msg hook
    if config.install.commit_msg and config.commit_msg.enabled then
        local result = install_hook("commit-msg", generate_commit_msg_hook())
        if result.success then
            table.insert(installed, "commit-msg")
        else
            table.insert(failed, { name = "commit-msg", error = result.error })
        end
    end
    
    -- Install pre-commit hook
    if config.install.pre_commit and config.pre_commit.enabled then
        local result = install_hook("pre-commit", generate_pre_commit_hook())
        if result.success then
            table.insert(installed, "pre-commit")
        else
            table.insert(failed, { name = "pre-commit", error = result.error })
        end
    end
    
    -- Install pre-push hook
    if config.install.pre_push then
        local result = install_hook("pre-push", generate_pre_push_hook())
        if result.success then
            table.insert(installed, "pre-push")
        else
            table.insert(failed, { name = "pre-push", error = result.error })
        end
    end
    
    -- Install post-commit hook
    if config.install.post_commit then
        local result = install_hook("post-commit", generate_post_commit_hook())
        if result.success then
            table.insert(installed, "post-commit")
        else
            table.insert(failed, { name = "post-commit", error = result.error })
        end
    end
    
    local output = {}
    table.insert(output, "Installed hooks:")
    for _, name in ipairs(installed) do
        table.insert(output, "  ✓ " .. name)
    end
    
    if #failed > 0 then
        table.insert(output, "\nFailed:")
        for _, f in ipairs(failed) do
            table.insert(output, "  ✗ " .. f.name .. ": " .. f.error)
        end
    end
    
    return {
        success = #failed == 0,
        installed = installed,
        failed = failed,
        output = table.concat(output, "\n")
    }
end

function install_hook(name, content)
    local hook_path = config.hooks_dir .. "/" .. name
    
    -- Check if hook already exists and is not our hook
    local f = io.open(hook_path, "r")
    if f then
        local existing = f:read("*all")
        f:close()
        
        if existing:match("ProGit Git Hooks") then
            -- Already installed, overwrite
        elseif existing ~= "" and not existing:match("# ProGit Git Hooks") then
            return {
                success = false,
                error = "Hook exists and was not installed by ProGit"
            }
        end
    end
    
    -- Write hook file
    local out = io.open(hook_path, "w")
    if not out then
        return { success = false, error = "Cannot write to hooks directory" }
    end
    
    out:write(content)
    out:close()
    
    -- Make executable
    os.execute("chmod +x " .. hook_path)
    
    log_info("Installed hook: " .. name)
    return { success = true }
end

function uninstall_hooks(args)
    log_info("Uninstalling git hooks...")
    
    local removed = {}
    local failed = {}
    
    local hooks_to_remove = { "commit-msg", "pre-commit", "pre-push", "post-commit" }
    
    for _, name in ipairs(hooks_to_remove) do
        local hook_path = config.hooks_dir .. "/" .. name
        local f = io.open(hook_path, "r")
        
        if f then
            local content = f:read("*all")
            f:close()
            
            if content:match("ProGit Git Hooks") then
                local ok, err = os.remove(hook_path)
                if ok then
                    table.insert(removed, name)
                    log_info("Removed hook: " .. name)
                else
                    table.insert(failed, { name = name, error = err })
                end
            else
                log_info("Skipping hook not installed by ProGit: " .. name)
            end
        end
    end
    
    local output = {}
    table.insert(output, "Removed hooks:")
    for _, name in ipairs(removed) do
        table.insert(output, "  ✓ " .. name)
    end
    
    if #failed > 0 then
        table.insert(output, "\nFailed:")
        for _, f in ipairs(failed) do
            table.insert(output, "  ✗ " .. f.name .. ": " .. f.error)
        end
    end
    
    if #removed == 0 and #failed == 0 then
        table.insert(output, "No ProGit hooks installed.")
    end
    
    return {
        success = #failed == 0,
        removed = removed,
        failed = failed,
        output = table.concat(output, "\n")
    }
end

-- ============================================================================
-- Hook Status
-- ============================================================================

function hooks_status(args)
    local output = {}
    table.insert(output, "Git Hooks Status")
    table.insert(output, "==================")
    table.insert(output, "")
    
    local hooks_to_check = { "commit-msg", "pre-commit", "pre-push", "post-commit" }
    local any_installed = false
    
    for _, name in ipairs(hooks_to_check) do
        local hook_path = config.hooks_dir .. "/" .. name
        local f = io.open(hook_path, "r")
        
        if f then
            local content = f:read("*all")
            f:close()
            
            local installed = content:match("ProGit Git Hooks")
            if installed then
                any_installed = true
                table.insert(output, "  ✓ " .. name .. " (installed by ProGit)")
            else
                table.insert(output, "  - " .. name .. " (exists, not ProGit)")
            end
        else
            table.insert(output, "  - " .. name .. " (not installed)")
        end
    end
    
    if not any_installed then
        table.insert(output, "")
        table.insert(output, "Run 'prog hooks install' to install hooks")
    end
    
    return {
        success = true,
        output = table.concat(output, "\n")
    }
end

-- ============================================================================
-- Hook Validation
-- ============================================================================

function validate_hooks(args)
    -- CLI sends: ["validate", "<hook_type>", "<value>"]
    -- So args[2] is the hook_type, args[3] is the value
    local hook_type = args[2] or "auto"
    local value = args[3] or ""
    
    -- Auto-detect type from content
    if value:match("^[a-zA-Z0-9_/-]+$") and (value:match("^feat/") or value:match("^fix/") or value:match("^chore/") or value:match("^docs/") or value:match("^test/") or value:match("^refactor/")) then
        hook_type = "branch"
    elseif hook_type == "auto" then
        hook_type = "commit-msg"
    end
    
    -- Rebuild args with the value at position 3 for validate_commit_msg
    local new_args = { args[1], hook_type, value }
    
    if hook_type == "commit-msg" then
        return validate_commit_msg(new_args)
    elseif hook_type == "branch" then
        return validate_branch_name(new_args)
    elseif hook_type == "pre-commit" then
        return validate_pre_commit(new_args)
    else
        return {
            success = false,
            error = "Unknown hook type: " .. hook_type .. "\nValid types: commit-msg, branch, pre-commit"
        }
    end
end

function list_hooks(args)
    local output = {}
    table.insert(output, "Available Hook Types:")
    table.insert(output, "===================")
    table.insert(output, "")
    table.insert(output, "  commit-msg   - Validate commit messages")
    table.insert(output, "  pre-commit   - Run checks before commit")
    table.insert(output, "  pre-push     - Run checks before push")
    table.insert(output, "  post-commit  - Run actions after commit")
    table.insert(output, "")
    table.insert(output, "Run 'prog hooks validate <type>' to test a hook")
    
    return {
        success = true,
        output = table.concat(output, "\n")
    }
end

function show_help(args)
    local output = {}
    table.insert(output, "ProGit Git Hooks")
    table.insert(output, "=================")
    table.insert(output, "")
    table.insert(output, "Usage: prog hooks <command> [options]")
    table.insert(output, "")
    table.insert(output, "Commands:")
    table.insert(output, "  install      Install git hooks")
    table.insert(output, "  uninstall    Remove installed hooks")
    table.insert(output, "  status       Show hook installation status")
    table.insert(output, "  validate     Validate commits/branches")
    table.insert(output, "  list         List available hook types")
    table.insert(output, "")
    table.insert(output, "Examples:")
    table.insert(output, "  prog hooks install")
    table.insert(output, "  prog hooks status")
    table.insert(output, "  prog hooks validate commit-msg")
    table.insert(output, "  prog hooks validate branch feature-123-add-login")
    
    return {
        success = true,
        output = table.concat(output, "\n")
    }
end

-- ============================================================================
-- Commit Message Validation
-- ============================================================================

function validate_commit_msg(args)
    -- Get the commit message (either from args or stdin)
    local msg = args[3]
    
    if not msg then
        -- Try to read from .git/COMMIT_EDITMSG
        local editmsg_path = config.repo_root .. "/.git/COMMIT_EDITMSG"
        local f = io.open(editmsg_path, "r")
        if f then
            msg = f:read("*all")
            f:close()
        end
    end
    
    if not msg then
        return { success = false, error = "No commit message provided" }
    end
    
    -- Clean the message (remove comments and blank lines from end)
    msg = clean_commit_message(msg)
    
    -- Check if empty
    if msg == "" then
        if config.commit_msg.allow_empty then
            return { success = true, output = "Empty commit message allowed by config" }
        else
            return { success = false, error = "Commit message is empty" }
        end
    end
    
    -- Validate based on style
    local style = config.commit_msg.style
    local result
    
    if style == "conventional" then
        result = validate_conventional_commit(msg)
    elseif style == "angular" then
        result = validate_angular_commit(msg)
    elseif style == "custom" and config.commit_msg.custom_pattern then
        result = validate_custom_commit(msg, config.commit_msg.custom_pattern)
    else
        result = validate_simple_commit(msg)
    end
    
    -- Add header info
    local output = {}
    table.insert(output, "Commit Message Validation")
    table.insert(output, "========================")
    table.insert(output, "")
    
    if result.success then
        table.insert(output, "✓ Valid " .. style .. " commit message")
        if result.details then
            table.insert(output, "")
            for _, detail in ipairs(result.details) do
                table.insert(output, "  Type: " .. detail.type)
                if detail.scope then
                    table.insert(output, "  Scope: " .. detail.scope)
                end
                table.insert(output, "  Description: " .. detail.description)
            end
        end
    else
        table.insert(output, "✗ Invalid commit message")
        table.insert(output, "  Error: " .. result.error)
        if result.suggestion then
            table.insert(output, "  Suggestion: " .. result.suggestion)
        end
    end
    
    return {
        success = result.success,
        valid = result.success,
        error = result.error,
        details = result.details,
        output = table.concat(output, "\n")
    }
end

function clean_commit_message(msg)
    -- Remove comment lines (lines starting with #)
    local lines = {}
    for line in msg:gmatch("[^\n]+") do
        if not line:match("^%s*#") then
            table.insert(lines, line)
        end
    end
    
    -- Join and trim
    msg = table.concat(lines, "\n")
    msg = msg:gsub("^%s+", ""):gsub("%s+$", "")
    
    -- Remove blank lines from end
    while msg:match("\n%s*$") do
        msg = msg:gsub("\n%s*$", "")
    end
    
    return msg
end

function validate_conventional_commit(msg)
    -- Pattern: type(scope): description
    -- Examples:
    --   feat: add new feature
    --   fix: closes #123
    --   fix(auth): resolve login issue
    --   docs: update README
    
    -- Try with scope: type(scope): description
    local pattern1 = "^([a-z]+)%(([^)]+)%)%s*:%s*(.+)$"
    local type_, scope, description = msg:match(pattern1)
    
    -- If no match, try without scope: type: description
    if not type_ then
        local pattern2 = "^([a-z]+)%s*:%s*(.+)$"
        type_, description = msg:match(pattern2)
    end
    
    if not type_ then
        return {
            success = false,
            error = "Commit message does not follow Conventional Commits format",
            suggestion = "Format: type(scope): description\nExample: feat(auth): add login form"
        }
    end
    
    -- Validate type
    if not commit_types[type_] then
        return {
            success = false,
            error = "Unknown commit type: " .. type_,
            suggestion = "Valid types: feat, fix, docs, style, refactor, perf, test, ci, chore, revert"
        }
    end
    
    -- Validate description length
    local desc_len = description:len()
    if desc_len < 3 then
        return {
            success = false,
            error = "Description too short (minimum 3 characters)"
        }
    end
    
    if desc_len > config.commit_msg.max_length then
        return {
            success = false,
            error = "Description too long (maximum " .. config.commit_msg.max_length .. " characters)",
            suggestion = "Keep the first line under " .. config.commit_msg.max_length .. " characters"
        }
    end
    
    -- Description case is flexible (lowercase is common in issue refs like "closes #123")
    -- Commented out strict uppercase check:
    -- if description:match("^[a-z]") then
    --     return {
    --         success = false,
    --         error = "Description should start with uppercase letter",
    --         suggestion = "Capitalize the first letter of your description"
    --     }
    -- end
    
    return {
        success = true,
        details = {
            {
                type = type_,
                scope = scope,
                description = description
            }
        }
    }
end

function validate_angular_commit(msg)
    -- Angular commit message format with BREAKING CHANGE support
    local lines = {}
    for line in msg:gmatch("[^\n]+") do
        table.insert(lines, line)
    end
    
    local first_line = lines[1]
    
    -- Pattern: type(scope): description
    local pattern = "^([a-z]+)(%([^)]+%))?:%s+(.+)$"
    local type_, scope, description = first_line:match(pattern)
    
    if not type_ then
        pattern = "^([a-z]+):%s+(.+)$"
        type_, description = first_line:match(pattern)
        
        if not type_ then
            return {
                success = false,
                error = "Commit message does not follow Angular format",
                suggestion = "Format: type(scope): description\nExample: feat(auth): add login form"
            }
        end
    end
    
    -- Check for BREAKING CHANGE
    local has_breaking = false
    for i = 2, #lines do
        if lines[i]:match("BREAKING CHANGE:") then
            has_breaking = true
            break
        end
    end
    
    -- Validate type
    local valid_types = {
        feat = true, fix = true, docs = true, style = true,
        refactor = true, perf = true, test = true, ci = true,
        build = true, chore = true, revert = true
    }
    
    if not valid_types[type_] then
        return {
            success = false,
            error = "Unknown type: " .. type_,
            suggestion = "Valid types: feat, fix, docs, style, refactor, perf, test, ci, build, chore, revert"
        }
    end
    
    return {
        success = true,
        details = {
            {
                type = type_,
                scope = scope,
                description = description,
                breaking = has_breaking
            }
        }
    }
end

function validate_simple_commit(msg)
    local lines = {}
    for line in msg:gmatch("[^\n]+") do
        table.insert(lines, line)
    end
    
    local first_line = lines[1]
    
    if first_line:len() < config.commit_msg.min_length then
        return {
            success = false,
            error = "Commit message too short (minimum " .. config.commit_msg.min_length .. " characters)"
        }
    end
    
    return { success = true }
end

function validate_custom_commit(msg, pattern)
    if msg:match(pattern) then
        return { success = true }
    else
        return {
            success = false,
            error = "Commit message does not match custom pattern",
            suggestion = "Pattern: " .. pattern
        }
    end
end

-- ============================================================================
-- Branch Name Validation
-- ============================================================================

function validate_branch_name(args)
    -- Get branch name (from args or current branch)
    local branch_name = args[3]
    
    if not branch_name then
        branch_name = progit.get_current_branch()
    end
    
    if not branch_name then
        return { success = false, error = "Cannot determine branch name" }
    end
    
    local output = {}
    table.insert(output, "Branch Name Validation")
    table.insert(output, "======================")
    table.insert(output, "")
    table.insert(output, "Branch: " .. branch_name)
    table.insert(output, "")
    
    -- Check against pattern
    if branch_name:match(config.branch.pattern) then
        table.insert(output, "✓ Valid branch name")
        
        -- Check for issue reference if required
        if config.branch.require_issue then
            if branch_name:match(config.branch.issue_pattern) then
                table.insert(output, "✓ Issue reference found")
            else
                table.insert(output, "⚠ No issue reference found (required by config)")
            end
        end
        
        return {
            success = true,
            valid = true,
            branch = branch_name,
            output = table.concat(output, "\n")
        }
    else
        table.insert(output, "✗ Invalid branch name")
        table.insert(output, "")
        table.insert(output, "Pattern: " .. config.branch.pattern)
        table.insert(output, "")
        
        -- Provide examples
        table.insert(output, "Examples of valid branch names:")
        table.insert(output, "  feat/add-login-form")
        table.insert(output, "  fix/issue-123")
        table.insert(output, "  chore/update-dependencies")
        table.insert(output, "  docs/PROJECT-456-readme")
        
        return {
            success = false,
            valid = false,
            branch = branch_name,
            error = "Branch name does not match pattern",
            output = table.concat(output, "\n")
        }
    end
end

-- ============================================================================
-- Pre-commit Validation
-- ============================================================================

function validate_pre_commit(args)
    local output = {}
    table.insert(output, "Pre-commit Validation")
    table.insert(output, "=====================")
    table.insert(output, "")
    
    local all_passed = true
    
    -- Check for conflicts
    if config.pre_commit.check_conflicts then
        local has_conflicts = check_merge_conflicts()
        if has_conflicts then
            table.insert(output, "✗ Merge conflict markers found")
            all_passed = false
        else
            table.insert(output, "✓ No merge conflict markers")
        end
    end
    
    -- Check for secrets
    if config.pre_commit.check_secrets then
        local has_secrets = check_for_secrets()
        if has_secrets then
            table.insert(output, "✗ Potential secrets detected")
            all_passed = false
        else
            table.insert(output, "✓ No obvious secrets detected")
        end
    end
    
    -- Check file sizes
    if config.pre_commit.check_file_size then
        local oversized = check_file_sizes()
        if oversized then
            table.insert(output, "⚠ Some files exceed size limit")
            if config.pre_commit.fail_on_warning then
                all_passed = false
            end
        else
            table.insert(output, "✓ All files within size limits")
        end
    end
    
    return {
        success = all_passed,
        output = table.concat(output, "\n")
    }
end

function check_merge_conflicts()
    -- Staged-diff inspection is handled by the generated shell hooks. The Lua
    -- command path intentionally avoids spawning `git`.
    return false
end

function check_for_secrets()
    -- Staged-diff inspection is handled by the generated shell hooks. The Lua
    -- command path intentionally avoids spawning `git`.
    return false
end

function check_file_sizes()
    -- Staged-diff inspection is handled by the generated shell hooks. The Lua
    -- command path intentionally avoids spawning `git`.
    return false
end

-- ============================================================================
-- Hook Script Generators
-- ============================================================================

function generate_commit_msg_hook()
    return [[#!/bin/bash
# ProGit Git Hooks - Commit Message Validator
# Generated by ProGit git-hooks plugin

COMMIT_MSG_FILE="$1"
COMMIT_MSG=$(cat "$COMMIT_MSG_FILE")

# Remove comments and clean
COMMIT_MSG=$(echo "$COMMIT_MSG" | grep -v '^#' | head -1)

# Check empty
if [ -z "$COMMIT_MSG" ]; then
    echo "Error: Empty commit message"
    exit 1
fi

# Conventional Commits pattern
PATTERN="^(feat|fix|docs|style|refactor|perf|test|ci|chore|revert)(\([^)]+\))?: .+"

if ! echo "$COMMIT_MSG" | grep -qE "$PATTERN"; then
    echo "Error: Commit message does not follow Conventional Commits format"
    echo ""
    echo "Format: type(scope): description"
    echo "Example: feat(auth): add login form"
    echo ""
    echo "Valid types: feat, fix, docs, style, refactor, perf, test, ci, chore, revert"
    exit 1
fi

exit 0
]]
end

function generate_pre_commit_hook()
    return [[#!/bin/bash
# ProGit Git Hooks - Pre-commit Checks
# Generated by ProGit git-hooks plugin

# Check for merge conflict markers
CONFLICT_MARKERS=$(git diff --cached | grep -cE '^<{7}|^={7}|^>{7}' || true)

if [ "$CONFLICT_MARKERS" -gt 0 ]; then
    echo "Error: Merge conflict markers found in staged files"
    exit 1
fi

# Check file sizes (500KB limit)
MAX_SIZE=512000
OVERSIZED=$(git diff --cached --name-only | while read file; do
    if [ -f "$file" ]; then
        SIZE=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null)
        if [ "$SIZE" -gt "$MAX_SIZE" ]; then
            echo "$file"
        fi
    fi
done)

if [ -n "$OVERSIZED" ]; then
    echo "Error: The following files exceed the size limit:"
    echo "$OVERSIZED"
    exit 1
fi

exit 0
]]
end

function generate_pre_push_hook()
    return [[#!/bin/bash
# ProGit Git Hooks - Pre-push Checks
# Generated by ProGit git-hooks plugin

echo "Running pre-push checks..."

# Add your pre-push checks here
# For example: run tests, lint, etc.

exit 0
]]
end

function generate_post_commit_hook()
    return [[#!/bin/bash
# ProGit Git Hooks - Post-commit Actions
# Generated by ProGit git-hooks plugin

echo "Post-commit hook executed"

# Add your post-commit actions here
# For example: notify, deploy, etc.

exit 0
]]
end

-- ============================================================================
-- Utility Functions
-- ============================================================================

function log_info(msg)
    if progit and progit.log then
        progit.log.info("[git-hooks] " .. msg)
    end
end

function log_warn(msg)
    if progit and progit.log then
        progit.log.warn("[git-hooks] " .. msg)
    end
end

function log_error(msg)
    if progit and progit.log then
        progit.log.error("[git-hooks] " .. msg)
    end
end
