# sober-raccoon

Premium ProGit plugin for Sober governance checks.

`sober-raccoon` uses the host-provided `sober.run(action, opts)` capability. It does not spawn processes from Lua and does not receive arbitrary shell access. The command path forwards argv to the host, and the host only executes the `sober` binary.

## Commands

The plugin owns the `sober` command namespace under `prog plugin`. Anything after `sober` is forwarded to the Sober CLI:

```bash
prog plugin sober preflight --base HEAD
prog plugin sober route list
prog plugin sober report list
prog plugin sober assist readiness --target release
prog plugin sober --version
```

The premium cockpit command remains available under `sober-raccoon`:

```bash
prog plugin sober-raccoon status
prog plugin sober-raccoon route list
```

## Sober Project

Sober is a local-first repository governance assistant for agentic coding. It keeps repositories moving under owner rules with Git authority, deterministic preflight checks, model routing, private steering, stored evidence, managed hooks, recoverable background work, and explicit Forgejo/Gitea posting.

Project source: `https://git.sovereign-society.org/Sober/sober.git`

The ProGit marketplace listing uses the Sober daemon mascot asset from `../Sober/sober-dev/assets/sober-raccoon-daemon.png`.

## Events

Dispatch a custom plugin event:

```json
{
  "type": "Custom",
  "data": {
    "name": "sober-raccoon",
    "payload": {
      "action": "status",
      "base": "HEAD"
    }
  }
}
```

Supported local event actions:

- `status`
- `doctor`
- `preflight`
- `review-preview`
- `hygiene`
- `hooks`
- `cli`

## Premium Status

License: `LicenseRef-ProGit-Premium`.

Distribution is intended for the ProGit premium plugin registry. This source package is the local development copy used by the ProGit runtime tests and marketplace metadata.
