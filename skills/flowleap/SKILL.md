---
name: flowleap
description: Start here — the umbrella skill for the FlowLeap Patent AI CLI. Maps every command family (patent/USPTO/OPS/academic/NPL/legal/citation reads, analytics, OCR, claim analysis, one-call patent verbs, the tools facade, and the raw API escape hatch) and routes to the specialist skills. Trigger when a user asks an agent to use FlowLeap, query the FlowLeap backend, verify local or deployed FlowLeap API health, run patent research commands, or debug FlowLeap CLI/API behavior.
---

# FlowLeap CLI

Use `flowleap` as the command layer for the FlowLeap Patent AI backend.
This is the entry-point skill: it maps the CLI surface and points to the
specialist skills. Shared reference (auth, global flags, config, output
formats): `flowleap-shared`.

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
export FLOWLEAP_API_KEY=fl_pat_your_token_here
flowleap auth login --api-key fl_pat_your_token_here
flowleap --json api profile
```

All credentials are sent as `Authorization: Bearer …` — either a Clerk JWT
(from the `flowleap auth login` OAuth device flow: user code + verification
URL) or a personal API token (`fl_pat_…`). Mint long-lived tokens for
headless use:

```bash
flowleap auth create-token --name my-agent --store
flowleap auth tokens
flowleap auth revoke-token <id>
```

Details: the `flowleap-auth` skill.

## Provider Keys (BYOK)

Patent data may need the user's own EPO OPS / USPTO ODP keys. If a command
fails with a `providerKeysHint` (code `provider_keys_required` /
`provider_keys_invalid`): **stop — this needs a human** (browser signup).
Ask the user to run `flowleap setup`, or apply keys they give you with
`flowleap keys set …`. Details: the `flowleap-keys` skill.

```bash
flowleap --json keys test    # live per-provider verdicts
```

## Agent-First Tool Facade

Prefer `flowleap tools` when you want runtime-discoverable, uniformly-shaped
operations (see the `flowleap-tools` skill for the full inventory):

```bash
flowleap --json tools list
flowleap --json tools run get_patent_summary patent_number=EP1000000
```

## One-Call Patent Verbs

Ergonomic wrappers over the tools facade plus document utilities:

```bash
flowleap --json summary EP1000000                      # biblio + legal + family + term
flowleap --json timeline EP1000000                     # prosecution timeline
flowleap --json compare EP1000000 US5443036            # 2-10 patents side by side
flowleap figures EP1000000 --out figure.png --page 3   # drawings; metadata without --out
flowleap --json convert-number EP1000000 --to docdb    # epodoc|docdb|original
flowleap --json analytics --keyword battery --date-from 2015-01-01   # corpus trends
flowleap ocr ./office-action.pdf                       # PDF/image/docx text extraction
flowleap analyze-claim --file claim1.txt --focus elements   # claim decomposition
```

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

## Install Skills

Ship these skills to any agent's skills directory:

```bash
flowleap skills install              # → ~/.claude/skills
flowleap skills install --project    # → .claude/skills
flowleap skills install --dir <path> # any other agent
```

## Skill Map

- Shared reference: `flowleap-shared` (auth, flags, config), `flowleap-auth`, `flowleap-keys`
- Data sources: `flowleap-patent` (EPO CQL), `flowleap-uspto` (ODP Lucene), `flowleap-ops` (EPO documents), `flowleap-academic`, `flowleap-npl`, `flowleap-legal`, `flowleap-citation`, `flowleap-tools` (facade)
- Personas: `persona-patent-attorney`, `persona-ip-analyst`, `persona-researcher`, `persona-startup-founder`
- Recipes (search/analysis): `recipe-prior-art-search`, `recipe-patent-landscape`, `recipe-freedom-to-operate`, `recipe-claim-analysis`, `recipe-patent-to-report`, `recipe-academic-literature-review`
- Recipes (prosecution/litigation): `recipe-office-action-response`, `recipe-invalidity-analysis`, `recipe-infringement-charting`, `recipe-claim-drafting`, `recipe-invention-disclosure`, `recipe-audit-report`
