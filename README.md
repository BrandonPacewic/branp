<h1>
<p align="center">
  <br>branp
</h1>
</p>

## About

`branp` is a a cross platform, general purpose, command line tool developed for hyper-specific
use cases in my own workflow. It is not intended to be used by anyone other than myself. That being said,
if you find use for it, let me know how it goes.

## Install

MacOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/BrandonPacewic/branp/mega/scripts/install.sh | sh
```

Windows:

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://raw.githubusercontent.com/BrandonPacewic/branp/mega/scripts/install.ps1 | iex"
```

## Usage

Basic usage can be provided by simply running `bp` in your terminal with no arguments. This will
give you a list of all available subcommands. 

### Machine-readable worktree status

`bp worktree list --json` emits a versioned JSON object on stdout. The `worktrees` array is sorted
by absolute `path`, and object keys are emitted in the documented order below:

```json
{
  "schema_version": 1,
  "worktrees": [
    {
      "path": "/path/to/repo",
      "kind": "base",
      "branch": "mega",
      "detached": false,
      "head": "0123456789abcdef",
      "clean": true,
      "dirty": false,
      "base": true,
      "current": true,
      "scratch": false,
      "available": false,
      "reusable": false,
      "in_use": false,
      "in_use_reasons": [],
      "leased": false,
      "unverified": false,
      "remote": "ok",
      "pull_request": null
    }
  ]
}
```

`branch`, `head`, `remote`, and `pull_request` are `null` when not applicable or unavailable.
`remote` is `"ok"` or `"prunable"`, matching the human-readable status. `available` reports the
durable availability fact and can remain true alongside dirty or in-use facts. A scratch worktree
is safe to reuse only when `reusable` is true. Missing, corrupt, transitional, or quarantined
state is reported with `unverified: true`, `available: false`, and `reusable: false`. Diagnostics
remain on stderr.

## License

Copyright (c) Brandon Pacewic

SPDX-License-Identifier: MIT
