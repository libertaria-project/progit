# sober-raccoon

Premium ProGit plugin for Sober governance checks.

`sober-raccoon` uses the host-provided `sober.run(action, opts)` capability. It does not spawn processes from Lua and does not receive arbitrary shell access.

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

Supported actions:

- `status`
- `preflight`
- `review-preview`
- `hygiene`
- `hooks`

## Premium Status

License: `LicenseRef-ProGit-Premium`.

Distribution is intended for the ProGit premium plugin registry. This source package is the local development copy used by the ProGit runtime tests and marketplace metadata.
