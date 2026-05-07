#compdef prog
# progit shell completions - zsh

_progit() {
    local -a commands
    commands=(
        'issue:Manage issues'
        'mr:Manage merge requests'
        'plugin:Manage plugins'
        'hook:Manage git hooks'
        'trust:Manage trusted keys'
        'sync:Sync with remote'
        'rebase:Rebase editor'
        'review:Code review mode'
        'tui:Launch TUI'
        'init:Initialize project'
    )
    
    local -a plugin_cmds
    plugin_cmds=(
        'install:Install a plugin'
        'remove:Remove a plugin'
        'list:List installed plugins'
        'verify:Verify plugin signature'
        'search:Search marketplace'
        'update:Update plugin'
        'submit:Submit to marketplace'
        'info:Show plugin info'
    )
    
    local -a trust_cmds
    trust_cmds=(
        'add:Add trusted key'
        'list:List trusted keys'
        'remove:Remove trusted key'
    )
    
    local -a issue_cmds
    issue_cmds=(
        'list:List issues'
        'new:Create issue'
        'show:Show issue'
        'status:Change status'
        'block:Block issue'
        'due-date:Set due date'
    )
    
    local -a mr_cmds
    mr_cmds=(
        'list:List merge requests'
        'show:Show merge request'
        'branch:Create branch'
        'create:Create merge request'
    )

    _arguments -C \
        '1: :->command' \
        '2: :->subcommand' \
        '3: :->arg' \
        '*:: :->args'
    
    case $state in
        command)
            _describe 'command' commands
            ;;
        subcommand)
            case $words[2] in
                plugin)
                    _describe 'plugin command' plugin_cmds
                    ;;
                trust)
                    _describe 'trust command' trust_cmds
                    ;;
                issue)
                    _describe 'issue command' issue_cmds
                    ;;
                mr)
                    _describe 'mr command' mr_cmds
                    ;;
            esac
            ;;
        arg)
            case $words[2] in
                plugin)
                    if [[ $words[3] == @(install|verify|remove) ]]; then
                        local plugins_dir="${PROGIT_PLUGINS_DIR:-$HOME/.progit/plugins}"
                        if [[ -d "$plugins_dir" ]]; then
                            _values 'plugin' $(ls "$plugins_dir" 2>/dev/null)
                        fi
                    fi
                    ;;
                trust)
                    if [[ $words[3] == "add" ]]; then
                        _values 'keyid' '18a10eb52cf3c001'
                    fi
                    ;;
            esac
            ;;
    esac
}

_progit "$@"
