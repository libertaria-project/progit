#!/bin/bash
# progit shell completions - bash

_progit_complete() {
    local cur prev words cword
    _init_completion || return
    
    # Main commands
    local commands="issue mr plugin hook trust sync rebase review tui init"
    
    # Subcommands for plugin
    local plugin_cmds="install remove list verify search update submit info"
    
    # Subcommands for trust
    local trust_cmds="add list remove"
    
    # Subcommands for issue
    local issue_cmds="list new show status block due-date"
    
    # Subcommands for mr
    local mr_cmds="list show branch create"

    if [[ $cword -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
    elif [[ $cword -eq 2 ]]; then
        case "${words[1]}" in
            plugin)
                COMPREPLY=( $(compgen -W "$plugin_cmds" -- "$cur") )
                ;;
            trust)
                COMPREPLY=( $(compgen -W "$trust_cmds" -- "$cur") )
                ;;
            issue)
                COMPREPLY=( $(compgen -W "$issue_cmds" -- "$cur") )
                ;;
            mr)
                COMPREPLY=( $(compgen -W "$mr_cmds" -- "$cur") )
                ;;
        esac
    elif [[ $cword -eq 3 ]]; then
        case "${words[1]}" in
            plugin)
                if [[ "${words[2]}" == "install" || "${words[2]}" == "verify" ]]; then
                    # Suggest installed plugins
                    local plugins_dir="${PROGIT_PLUGINS_DIR:-$HOME/.progit/plugins}"
                    if [[ -d "$plugins_dir" ]]; then
                        COMPREPLY=( $(compgen -W "$(ls "$plugins_dir" 2>/dev/null)" -- "$cur") )
                    fi
                fi
                ;;
            trust)
                if [[ "${words[2]}" == "add" ]]; then
                    COMPREPLY=( $(compgen -W "18a10eb52cf3c001" -- "$cur") )
                fi
                ;;
        esac
    fi
} && complete -F _progit_complete prog
