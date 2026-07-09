# FlowLeap CLI

One CLI for FlowLeap Patent AI — built for humans and AI agents.

A Rust CLI for the [FlowLeap Patent AI](https://api.flowleap.co) backend API. Search patents, build queries, and explore academic literature — all from your terminal. Ships with Agent Skills (SKILL.md files) for seamless AI agent integration.

## Installation

**npm / pnpm / yarn:**
```bash
npm i -g flowleap
```

The npm package has no install scripts. The native binary is downloaded on
first run from the matching GitHub release, verified against the release's
sha256 `checksums.txt` before it executes, and the package is published with
[npm provenance](https://docs.npmjs.com/generating-provenance-statements)
attesting the exact repo, commit, and CI run that built it.

**Quick install (macOS / Linux):**
```bash
curl -fsSL https://raw.githubusercontent.com/abdullahatrash/flowleap-cli/main/install.sh | sh
```

**From source (requires Rust):**
```bash
cargo install --git https://github.com/abdullahatrash/flowleap-cli.git
```

## Quick Start

```bash
# Verify CLI setup and backend reachability
flowleap --json doctor

# Use the local backend during development
flowleap --json doctor --base-url http://localhost:8000

# Authenticate (opens browser for OAuth)
flowleap auth login

# Or mint a long-lived personal API token for headless/agent use
flowleap auth create-token --name my-agent --store

# One-time guided setup (auth + BYOK provider keys)
flowleap setup

# Search patents
flowleap patent search --query "solar panel efficiency"

# Build a CQL query from natural language; dry-run verifies request shape
flowleap patent build-query "patents about lithium battery recycling filed by Tesla" --dry-run

# Direct EPO OPS access
flowleap ops biblio EP1234567
flowleap ops claims EP1234567

# USPTO Open Data Portal
flowleap uspto search --query "wireless charging" --limit 3
flowleap uspto grant 11800000

# Search academic literature
flowleap academic search "machine learning patent classification"

# Search NPL, legal, and citation data
flowleap --json npl "battery thermal management" --limit 10
flowleap --json legal search "doctrine of equivalents" --limit 10
flowleap --json citation search 16000001 --size 20

# Raw API escape hatch
flowleap --json api request get /v1/health
flowleap --json api request post /v1/patent-search --body-file request.json --dry-run
```

## Smoke Tests

Use these commands after installing or publishing a new version. They are also good examples for agents because they use `--json` and make required flags explicit.

```bash
# Installed version
flowleap --version

# Production health and auth/config check
flowleap --json doctor --base-url https://api.flowleap.co

# Patent search requires --query
flowleap --json patent search --query "battery cooling system" --limit 3 --base-url https://api.flowleap.co
flowleap --json uspto search --query "wireless charging" --limit 3 --base-url https://api.flowleap.co

# These commands use positional text arguments
flowleap --json academic search "solid state battery electrolyte" --limit 3 --base-url https://api.flowleap.co
```

When debugging request shape, add `--dry-run`. This is also the safest way to test endpoints whose backend query contract may still be changing:

```bash
flowleap --json patent search --query "battery cooling system" --limit 1 --dry-run
flowleap --json uspto search --query "wireless charging" --limit 1 --base-url https://api.flowleap.co --dry-run
flowleap --json patent build-query "battery cooling system for electric vehicles" --base-url https://api.flowleap.co --dry-run
```

`--dry-run` shows the exact method, URL, auth status, and JSON body the CLI will send.

## Authentication

FlowLeap CLI supports three authentication methods (all sent as `Authorization: Bearer`):

1. **OAuth 2.0 Device Flow** (recommended for humans): `flowleap auth login` opens your browser
2. **Personal API token** (recommended for agents/CI): `flowleap auth create-token --name <n> --store` mints a revocable `fl_pat_…` token (list with `auth tokens`, revoke with `auth revoke-token`)
3. **Environment variables**: `FLOWLEAP_API_KEY` (an `fl_pat_…` token) or `FLOWLEAP_TOKEN`

Patent providers may also need your own keys (EPO OPS, USPTO ODP) — run `flowleap setup` or see `flowleap keys --help`.

Credentials are stored in `~/.config/flowleap/credentials.toml` (mode 0600).

```bash
# Check auth status
flowleap auth status

# Clear all credentials (including provider keys)
flowleap auth logout

# Clear only the OAuth session token (keep API key + provider keys)
flowleap auth logout --session-only
```

## Commands

| Command | Description |
|---------|-------------|
| `doctor` | Check config, auth, and backend reachability |
| `health` | Public backend health probes |
| `api request` | Raw authenticated API escape hatch |
| `auth login/logout/status` | Manage authentication |
| `patent search` | Search patents (EPO/USPTO) |
| `patent build-query` | Natural language → CQL query |
| `uspto` | USPTO ODP search, grants, applications, continuity, query builder |
| `ops` | Direct EPO OPS API (biblio, claims, family, legal, abstract) |
| `academic search` | Search academic literature |
| `npl` | Search non-patent literature |
| `legal` | Search patent-law documents |
| `citation` | Search USPTO citation/prior-art data |
| `config` | Manage CLI configuration |

## Configuration

Configuration is stored in `~/.config/flowleap/config.toml`.

```bash
# Set base URL
flowleap config set base-url https://api.flowleap.co

# Read a config value
flowleap config get base-url
```

### Config Precedence

CLI flags > environment variables > config file

### Environment Variables

| Variable | Description |
|----------|-------------|
| `FLOWLEAP_API_KEY` | API key for authentication |
| `FLOWLEAP_TOKEN` | Bearer token for authentication |
| `FLOWLEAP_BASE_URL` | API base URL |
| `FLOWLEAP_NO_UPDATE_CHECK` | Disable the once-a-day update notice (it is already skipped for `--json`, `--dry-run`, and non-TTY runs) |

## Global Flags

```
--json              Emit stable machine-readable JSON
--output <format>   Output format: json, table, human (default: human)
--base-url <url>    Override API base URL
--api-key <key>     Override stored API key
--token <token>     Override stored token
--dry-run           Show request details without executing
--verbose, -v       Show verbose request/response details
```

## AI Agent Integration

The repo ships Agent Skills (`SKILL.md` files) — one for every CLI command, plus personas and multi-step recipes. Skills are structured Markdown files that any LLM can read natively. They are baked into the binary, so `flowleap skills install` works anywhere.

### Installing Skills Per Harness

```bash
flowleap skills install                    # Claude Code (user): ~/.claude/skills
flowleap skills install --target claude-project   # Claude Code (project): ./.claude/skills
flowleap skills install --target codex     # Codex: marked block in ./AGENTS.md
flowleap skills install --target cursor    # Cursor: ./.cursor/rules/flowleap.mdc
flowleap skills install --target gemini    # Gemini CLI: marked block in ./GEMINI.md
flowleap skills install --dir <path>       # Raw SKILL.md copy anywhere
```

Claude targets copy the full SKILL.md directories; the other targets render a condensed agent-rules document (command reference + workflow triggers) generated from the same embedded skill content — the file each harness actually auto-loads.

Every install is stamped with the CLI version and recorded in the config file. After upgrading the CLI, refresh all recorded installs in one go:

```bash
flowleap skills update
```

The daily update notice also warns when installed skills are stale (rendered by an older CLI version).

### Skill Categories

| Category | Description |
|----------|-------------|
| **Service skills** (`flowleap-*`) | One per CLI command (patent, ops, etc.) |
| **Persona skills** (`persona-*`) | Role-based bundles (patent attorney, researcher, etc.) |
| **Recipe skills** (`recipe-*`) | Multi-step workflows (prior art search, FTO analysis, etc.) |

### Skills Directory

```
skills/                            # 28 skills, embedded in the binary
  flowleap/SKILL.md                # Start here: umbrella + skill map
  flowleap-shared/SKILL.md         # Auth, global flags, config reference
  flowleap-auth/SKILL.md           # OAuth device flow + fl_pat_ tokens
  flowleap-keys/SKILL.md           # BYOK provider keys (EPO OPS, USPTO ODP)
  flowleap-patent/SKILL.md         # EPO patent search + CQL query builder
  flowleap-uspto/SKILL.md          # USPTO ODP search, grants, continuity
  flowleap-ops/SKILL.md            # Direct EPO OPS document data
  flowleap-academic/SKILL.md       # Academic literature search
  flowleap-npl/SKILL.md            # Non-patent literature (OpenAlex)
  flowleap-legal/SKILL.md          # Patent-law reference search (RAG)
  flowleap-citation/SKILL.md       # USPTO enriched citation data
  flowleap-tools/SKILL.md          # Agent-first /v1/tools facade
  persona-patent-attorney/SKILL.md
  persona-ip-analyst/SKILL.md
  persona-researcher/SKILL.md
  persona-startup-founder/SKILL.md
  recipe-prior-art-search/SKILL.md
  recipe-patent-landscape/SKILL.md
  recipe-freedom-to-operate/SKILL.md
  recipe-claim-analysis/SKILL.md
  recipe-patent-to-report/SKILL.md
  recipe-academic-literature-review/SKILL.md
  recipe-office-action-response/SKILL.md    # prosecution
  recipe-claim-drafting/SKILL.md            # prosecution
  recipe-invention-disclosure/SKILL.md      # prosecution
  recipe-invalidity-analysis/SKILL.md       # litigation
  recipe-infringement-charting/SKILL.md     # litigation
  recipe-audit-report/SKILL.md              # governance
```

### Agent-Friendly Output

Always use `--output json` when integrating with AI agents for reliable parsing:

```bash
flowleap --json doctor
flowleap --json patent search --query "solar panel"
flowleap --json ops claims EP1234567
```

For local backend development:

```bash
flowleap --json doctor --base-url http://localhost:8000
flowleap --json health cache --base-url http://localhost:8000
```

Use `api request` only when a high-level command is missing:

```bash
flowleap --json api request get /v1/health
flowleap --json api request post /v1/patent-search --body '{"query":"solar","limit":1}' --dry-run
```

### AI Configuration Files

| File | Purpose |
|------|---------|
| `CLAUDE.md` | Entry point for Claude Code |
| `AGENTS.md` | Architecture and contribution guide for AI agents |
| `.claude/settings.json` | Claude Code configuration |

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy

# Format
cargo fmt
```

## License

MIT
