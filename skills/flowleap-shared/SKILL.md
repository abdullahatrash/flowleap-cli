---
name: flowleap-shared
description: Shared reference for every FlowLeap skill — authentication (OAuth device flow, fl_pat_ personal API tokens), credential storage, config precedence, global flags, and output formats. Trigger when a FlowLeap command needs credentials set up, a global flag explained, config file locations, or output-format guidance; for the overall command map start from the `flowleap` skill.
---

# FlowLeap CLI — Shared Reference

Shared authentication, configuration, and global-flag reference used by every
other FlowLeap skill. For the overall map of commands, skills, and workflows,
start from the `flowleap` skill.

## Authentication

Every authenticated request sends `Authorization: Bearer <credential>` — either
a session JWT from the OAuth device flow or a long-lived personal API token
(`fl_pat_…`).

```bash
# OAuth 2.0 device flow: prints a user code + verification URL, opens the
# browser, and polls until the login is approved
flowleap auth login

# Store a personal API token directly
flowleap auth login --api-key fl_pat_your_token_here

# Store a session token directly
flowleap auth login --token eyJhbGci...

# Mint a long-lived fl_pat_ token for headless/agent use (shown once)
flowleap auth create-token --name my-agent --store

# Check status
flowleap auth status

# Clear all credentials (including EPO/USPTO provider keys)
flowleap auth logout

# Clear only the OAuth session token (keep API key + provider keys)
flowleap auth logout --session-only
```

Environment variable overrides (highest priority):
- `FLOWLEAP_API_KEY` — personal API token (`fl_pat_…`)
- `FLOWLEAP_TOKEN` — Bearer token
- `FLOWLEAP_BASE_URL` — API base URL

Full auth details (token listing/revocation, 401 self-heal): the
`flowleap-auth` skill. Patent-provider keys (EPO OPS / USPTO ODP BYOK): the
`flowleap-keys` skill.

## Global Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--json` | Shorthand for `--output json` | `false` |
| `--output <format>` | Output format: `json`, `table`, `human` | `human` |
| `--base-url <url>` | API base URL | `https://api.flowleap.co` |
| `--api-key <key>` | Override stored API key (`fl_pat_…`) | — |
| `--token <token>` | Override stored token | — |
| `--dry-run` | Show request without executing | `false` |
| `--verbose`, `-v` | Show request/response details | `false` |

## Configuration

Config is stored in `~/.config/flowleap/config.toml`. Credentials live
separately in `~/.config/flowleap/credentials.toml` (written mode 0600).

```bash
flowleap config set base-url https://api.flowleap.co
flowleap config get base-url
```

## Config Precedence

CLI flags > environment variables > config file

## Output Formats

- `--json` (or `--output json`) — Machine-readable JSON (best for agents)
- `--output table` — Formatted table
- `--output human` — Human-readable text (default)

When using FlowLeap as an AI agent, always pass `--json` for reliable parsing.

## Safety

- Use `--dry-run` before executing mutating operations
- Use `--verbose` to inspect request details (credentials are redacted)
- Never include credentials in commit messages or logs
