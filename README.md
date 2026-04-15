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
# Authenticate (opens browser for OAuth)
flowleap auth login

# Or use an API key directly
flowleap auth login --api-key sk-...

# Search patents
flowleap patent search --query "solar panel efficiency"

# Build a CQL query from natural language
flowleap patent build-query "patents about lithium battery recycling filed by Tesla"

# Direct EPO OPS access
flowleap ops biblio EP1234567
flowleap ops claims EP1234567

# Search academic literature
flowleap academic search "machine learning patent classification"
```

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
| `auth login/logout/status` | Manage authentication |
| `patent search` | Search patents (EPO/USPTO) |
| `patent build-query` | Natural language → CQL query |
| `ops` | Direct EPO OPS API (biblio, claims, family, legal, abstract) |
| `academic search` | Search academic literature |
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
flowleap patent search --query "solar panel" --output json
flowleap ops claims EP1234567 --output json
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
