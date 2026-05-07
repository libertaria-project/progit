# progit shell completions - fish

complete -c prog -f -n '__fish_use_subcommand' -a 'issue' -d 'Manage issues'
complete -c prog -f -n '__fish_use_subcommand' -a 'mr' -d 'Manage merge requests'
complete -c prog -f -n '__fish_use_subcommand' -a 'plugin' -d 'Manage plugins'
complete -c prog -f -n '__fish_use_subcommand' -a 'hook' -d 'Manage git hooks'
complete -c prog -f -n '__fish_use_subcommand' -a 'trust' -d 'Manage trusted keys'
complete -c prog -f -n '__fish_use_subcommand' -a 'sync' -d 'Sync with remote'
complete -c prog -f -n '__fish_use_subcommand' -a 'rebase' -d 'Rebase editor'
complete -c prog -f -n '__fish_use_subcommand' -a 'review' -d 'Code review mode'
complete -c prog -f -n '__fish_use_subcommand' -a 'tui' -d 'Launch TUI'
complete -c prog -f -n '__fish_use_subcommand' -a 'init' -d 'Initialize project'

# plugin subcommands
complete -c prog -f -n '__fish_seen_subcommand_from plugin' -a 'install' -d 'Install a plugin'
complete -c prog -f -n '__fish_seen_subcommand_from plugin' -a 'remove' -d 'Remove a plugin'
complete -c prog -f -n '__fish_seen_subcommand_from plugin' -a 'list' -d 'List installed plugins'
complete -c prog -f -n '__fish_seen_subcommand_from plugin' -a 'verify' -d 'Verify plugin signature'
complete -c prog -f -n '__fish_seen_subcommand_from plugin' -a 'search' -d 'Search marketplace'
complete -c prog -f -n '__fish_seen_subcommand_from plugin' -a 'update' -d 'Update plugin'
complete -c prog -f -n '__fish_seen_subcommand_from plugin' -a 'submit' -d 'Submit to marketplace'
complete -c prog -f -n '__fish_seen_subcommand_from plugin' -a 'info' -d 'Show plugin info'

# trust subcommands
complete -c prog -f -n '__fish_seen_subcommand_from trust' -a 'add' -d 'Add trusted key'
complete -c prog -f -n '__fish_seen_subcommand_from trust' -a 'list' -d 'List trusted keys'
complete -c prog -f -n '__fish_seen_subcommand_from trust' -a 'remove' -d 'Remove trusted key'

# issue subcommands
complete -c prog -f -n '__fish_seen_subcommand_from issue' -a 'list' -d 'List issues'
complete -c prog -f -n '__fish_seen_subcommand_from issue' -a 'new' -d 'Create issue'
complete -c prog -f -n '__fish_seen_subcommand_from issue' -a 'show' -d 'Show issue'
complete -c prog -f -n '__fish_seen_subcommand_from issue' -a 'status' -d 'Change status'
complete -c prog -f -n '__fish_seen_subcommand_from issue' -a 'block' -d 'Block issue'
complete -c prog -f -n '__fish_seen_subcommand_from issue' -a 'due-date' -d 'Set due date'

# mr subcommands
complete -c prog -f -n '__fish_seen_subcommand_from mr' -a 'list' -d 'List merge requests'
complete -c prog -f -n '__fish_seen_subcommand_from mr' -a 'show' -d 'Show merge request'
complete -c prog -f -n '__fish_seen_subcommand_from mr' -a 'branch' -d 'Create branch'
complete -c prog -f -n '__fish_seen_subcommand_from mr' -a 'create' -d 'Create merge request'

# Plugin names for install/verify/remove
complete -c prog -f -n '__fish_seen_subcommand_from plugin; and __fish_seen_subcommand_from install' -a 'syntax-highlight' -d 'Syntax highlighting for diffs'
complete -c prog -f -n '__fish_seen_subcommand_from plugin; and __fish_seen_subcommand_from install' -a 'jira-sync' -d 'Bidirectional Jira sync'
complete -c prog -f -n '__fish_seen_subcommand_from plugin; and __fish_seen_subcommand_from install' -a 'gitlab-ci' -d 'GitLab CI/CD status'
complete -c prog -f -n '__fish_seen_subcommand_from plugin; and __fish_seen_subcommand_from install' -a 'slack-notify' -d 'Slack notifications'
complete -c prog -f -n '__fish_seen_subcommand_from plugin; and __fish_seen_subcommand_from install' -a 'csv-export' -d 'Export issues to CSV'
complete -c prog -f -n '__fish_seen_subcommand_from plugin; and __fish_seen_subcommand_from install' -a 'symbio-expert' -d 'AI-powered analysis'
complete -c prog -f -n '__fish_seen_subcommand_from plugin; and __fish_seen_subcommand_from install' -a 'forgejo-notify' -d 'Forgejo notifications'
complete -c prog -f -n '__fish_seen_subcommand_from plugin; and __fish_seen_subcommand_from install' -a 'git-hooks' -d 'Git hooks manager'

# KeyID for trust add
complete -c prog -f -n '__fish_seen_subcommand_from trust; and __fish_seen_subcommand_from add' -a '18a10eb52cf3c001' -d 'ProGit Core Team'
