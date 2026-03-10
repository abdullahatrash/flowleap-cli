# FlowLeap CLI

One CLI for FlowLeap Patent AI — built for humans and AI agents.

A Rust CLI for the [FlowLeap Patent AI](https://api.flowleap.co) backend API. Search patents, chat with AI models, run OCR, and more — all from your terminal. Ships with 20+ Agent Skills (SKILL.md files) for seamless AI agent integration.

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Authenticate (opens browser for OAuth)
flowleap auth login

# Or use an API key directly
flowleap auth login --api-key sk-...

# Chat with an AI model
flowleap chat "What is claim 1 of EP1234567?"

# Pipe input
echo "Summarize this patent" | flowleap chat

# Search patents
flowleap patent search --query "solar panel efficiency"

# Build a CQL query from natural language
flowleap patent build-query "patents about lithium battery recycling filed by Tesla"

# List available models
flowleap models

# OCR a document
flowleap ocr extract document.pdf

# Search academic literature
flowleap academic search "machine learning patent classification"
```

## Authentication

FlowLeap CLI supports three authentication methods:

1. **OAuth 2.0 + PKCE** (recommended): `flowleap auth login` opens your browser
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
| `chat` | Chat with AI models (SSE streaming) |
| `patent search` | Search patents (EPO/USPTO) |
| `patent build-query` | Natural language → CQL query |
| `models` | List available AI models |
| `ocr extract` | Extract text from PDF/images |
| `academic search` | Search academic literature |
| `ops` | Direct EPO OPS API access |
| `config` | Manage CLI configuration |
| `schema` | Discover available services |

## Configuration

Configuration is stored in `~/.config/flowleap/config.toml`.

```bash
# Set default model
flowleap config set default-model patent-claude-sonnet

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

The repo ships 20+ Agent Skills (`SKILL.md` files) — one for every CLI command, plus personas and multi-step recipes. Skills are structured Markdown files that any LLM can read natively.

### Skill Categories

| Category | Count | Description |
|----------|-------|-------------|
| **Service skills** (`flowleap-*`) | 8 | One per CLI command (chat, patent, ocr, etc.) |
| **Persona skills** (`persona-*`) | 4 | Role-based bundles (patent attorney, researcher, etc.) |
| **Recipe skills** (`recipe-*`) | 6 | Multi-step workflows (prior art search, FTO analysis, etc.) |

### Skills Directory

```
skills/
  flowleap-shared/SKILL.md        # Auth, global flags, common patterns
  flowleap-auth/SKILL.md          # Authentication commands
  flowleap-chat/SKILL.md          # AI chat completions
  flowleap-patent/SKILL.md        # Patent search + query builder
  flowleap-ocr/SKILL.md           # OCR document processing
  flowleap-academic/SKILL.md      # Academic literature search
  flowleap-models/SKILL.md        # List AI models
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
flowleap chat --no-stream --output json "Analyze this patent"
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
