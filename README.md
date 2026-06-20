# FlowLeap CLI

One CLI for FlowLeap Patent AI — built for humans and AI agents.

A Rust CLI for the [FlowLeap Patent AI](https://api.flowleap.co) backend API. Search patents, build queries, and explore academic literature — all from your terminal. Ships with Agent Skills (SKILL.md files) for seamless AI agent integration.

## Installation

**npm / pnpm / yarn:**
```bash
npm i -g flowleap
```

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

# Or use an API key directly
flowleap auth login --api-key sk-...

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

FlowLeap CLI supports three authentication methods:

1. **OAuth 2.0 Device Flow** (recommended): `flowleap auth login` opens your browser
2. **API key**: `flowleap auth login --api-key sk-...`
3. **Environment variables**: `FLOWLEAP_API_KEY` or `FLOWLEAP_TOKEN`

Credentials are stored in `~/.config/flowleap/credentials.toml`.

```bash
# Check auth status
flowleap auth status

# Clear credentials
flowleap auth logout
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

# List all config
flowleap config list

# Reset to defaults
flowleap config reset
```

### Config Precedence

CLI flags > environment variables > config file

### Environment Variables

| Variable | Description |
|----------|-------------|
| `FLOWLEAP_API_KEY` | API key for authentication |
| `FLOWLEAP_TOKEN` | Bearer token for authentication |
| `FLOWLEAP_BASE_URL` | API base URL |

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

The repo ships Agent Skills (`SKILL.md` files) — one for every CLI command, plus personas and multi-step recipes. Skills are structured Markdown files that any LLM can read natively.

### Skill Categories

| Category | Description |
|----------|-------------|
| **Service skills** (`flowleap-*`) | One per CLI command (patent, ops, etc.) |
| **Persona skills** (`persona-*`) | Role-based bundles (patent attorney, researcher, etc.) |
| **Recipe skills** (`recipe-*`) | Multi-step workflows (prior art search, FTO analysis, etc.) |

### Skills Directory

```
skills/
  flowleap-shared/SKILL.md        # Auth, global flags, common patterns
  flowleap-auth/SKILL.md          # Authentication commands
  flowleap-patent/SKILL.md        # Patent search + query builder
  flowleap-academic/SKILL.md      # Academic literature search
  flowleap-ops/SKILL.md           # Direct EPO OPS API
  persona-patent-attorney/SKILL.md
  persona-researcher/SKILL.md
  persona-startup-founder/SKILL.md
  persona-ip-analyst/SKILL.md
  recipe-prior-art-search/SKILL.md
  recipe-patent-landscape/SKILL.md
  recipe-patent-to-report/SKILL.md
  recipe-claim-analysis/SKILL.md
  recipe-freedom-to-operate/SKILL.md
  recipe-academic-literature-review/SKILL.md
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
