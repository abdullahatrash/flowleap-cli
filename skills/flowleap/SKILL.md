---
name: flowleap
description: Use the installed FlowLeap CLI to inspect FlowLeap Patent AI backend health, authenticate safely, run patent/USPTO/OPS/academic/NPL/legal/citation reads, and use the raw API escape hatch. Trigger when a user asks an agent to use FlowLeap, query the FlowLeap backend, verify local or deployed FlowLeap API health, run patent research commands, or debug FlowLeap CLI/API behavior.
---

# FlowLeap CLI

Use `flowleap` as the command layer for the FlowLeap Patent AI backend. Prefer installed `flowleap` on `PATH`; when working inside this repo before installation, use `target/debug/flowleap` after `cargo build`.

## Start Here

```bash
command -v flowleap || true
flowleap --json doctor
```

For local backend work:

```bash
flowleap --json doctor --base-url http://localhost:8000
flowleap --json health --base-url http://localhost:8000
flowleap --json health cache --base-url http://localhost:8000
```

For production:

```bash
flowleap --json doctor --base-url https://api.flowleap.co
```

## Auth

Use env/config auth; avoid credential flags except for explicit one-off tests.

```bash
export FLOWLEAP_API_KEY=...
flowleap auth login --api-key ...
flowleap --json api profile
```

Org API keys beginning with `fl_org_` are sent as `X-API-Key`. Other API keys/tokens use bearer auth.

## Safe Read Workflow

Use `--json` for agent parsing. Use `--dry-run` before protected calls when auth or request shape is uncertain.

```bash
flowleap --json patent search --query "solar panel efficiency" --limit 10
flowleap --json uspto search --query "wireless charging" --limit 10
flowleap --json ops biblio EP1234567
flowleap --json academic search "machine learning patent classification" --limit 10
flowleap --json npl "lithium-ion battery thermal management" --limit 10
flowleap --json legal search "doctrine of equivalents" --limit 10
flowleap --json citation search 16000001 --size 20
```

Argument shapes differ by command:

```bash
# These require --query
flowleap --json patent search --query "battery cooling system" --limit 3
flowleap --json uspto search --query "wireless charging" --limit 3

# These use positional text arguments
flowleap --json patent build-query "battery cooling system for electric vehicles" --dry-run
flowleap --json academic search "solid state battery electrolyte" --limit 3
```

When checking whether the CLI is sending the intended request, prefer `--dry-run`:

```bash
flowleap --json patent search --query "battery cooling system" --limit 1 --dry-run
```

If dry-run shows the expected JSON body but the live response shape or count differs, treat that as backend behavior to investigate separately.

## Raw Escape Hatch

Use high-level commands first. Use raw requests only when a route is missing or while debugging backend behavior.

```bash
flowleap --json api request get /v1/health
flowleap --json api request get /health/cache --base-url http://localhost:8000
flowleap --json api request post /v1/patent-search --body-file request.json --dry-run
```

Do not run raw `post`, `put`, `patch`, or `delete` against a live service unless the user asked for that specific write. Prefer `--dry-run` first.

## Install And Validate

```bash
make install-local
command -v flowleap
flowleap --json doctor
```

Required repo checks before publishing CLI changes:

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```
