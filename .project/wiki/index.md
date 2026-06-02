# ProGit

ProGit is repository-centric. The repository is the system of record; identity is metadata and policy input.

This wiki is the minimal project-owned entrypoint for ProGit-aware remotes. It gives the TUI, local adapters, and the future ProGit Remote a stable Markdown surface to render without pulling project truth into an external database.

## Core Contract

- `.project/config.kdl` defines project configuration.
- `.project/policy.kdl` defines core enforcement posture.
- `.project/plugins.kdl` defines plugin trust and capability grants.
- `.project/schemas/manifest.kdl` declares project-owned contract schemas.
- `.project/wiki/manifest.kdl` declares renderable Markdown entrypoints.
