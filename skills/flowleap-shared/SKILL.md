---
name: flowleap-shared
version: 1.0.0
description: "FlowLeap CLI: Shared authentication, configuration, and global flags."
metadata:
  category: "patent-ai"
  requires:
    bins: ["flowleap"]
  cliHelp: "flowleap --help"
---

# FlowLeap CLI — Shared Configuration

Read this skill first before using any other FlowLeap skill.

## Installation

```bash
cargo install --path .
```

## Authentication

Authenticate before using any command that requires API access.

```bash
# OAuth 2.0 + PKCE (opens browser)
flowleap auth login

# Direct API key
flowleap auth login --api-key sk-...

# Direct token
flowleap auth login --token eyJ...

# Check status
flowleap auth status

# Clear credentials
flowleap auth logout
```

Environment variable overrides (highest priority):
- `FLOWLEAP_API_KEY` — API key
- `FLOWLEAP_TOKEN` — Bearer token
- `FLOWLEAP_BASE_URL` — API base URL

## Global Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--output <format>` | Output format: `json`, `table`, `human` | `human` |
| `--base-url <url>` | API base URL | `https://api.flowleap.co` |
| `--api-key <key>` | Override stored API key | — |
| `--token <token>` | Override stored token | — |
| `--dry-run` | Show request without executing | `false` |
| `--verbose`, `-v` | Show request/response details | `false` |

## Configuration

Config stored in `~/.config/flowleap/config.toml`. Credentials in `~/.config/flowleap/credentials.toml`.

```bash
flowleap config set default-model patent-claude-sonnet
flowleap config set base-url https://api.flowleap.dev
flowleap config list
flowleap config reset
```

## Config Precedence

CLI flags > environment variables > config file

## Output Formats

- `--output json` — Machine-readable JSON (best for agents)
- `--output table` — Formatted table
- `--output human` — Human-readable text (default)

When using FlowLeap as an AI agent, always use `--output json` for reliable parsing.

## Safety

- Use `--dry-run` before executing mutating operations
- Use `--verbose` to inspect request details
- Never include credentials in commit messages or logs
