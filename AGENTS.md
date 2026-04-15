# FlowLeap CLI — Agent & Contributor Guide

## Overview

`flowleap` is a Rust CLI for the FlowLeap Patent AI backend API. It provides patent search, CQL query building, academic search, and direct EPO OPS access — designed for both human users and AI agents.

## Build & Test

```bash
cargo build              # Build the binary
cargo test               # Run all tests
cargo clippy             # Lint (must pass with zero warnings)
cargo fmt --check        # Format check
```

All four must pass before submitting changes.

## Architecture

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point, clap argument parsing, command routing |
| `src/config.rs` | TOML config (`config.toml`) and credentials (`credentials.toml`) management |
| `src/client.rs` | HTTP client context — auth injection, request building, dry-run/verbose |
| `src/output.rs` | Output module (re-exports formatter) |
| `src/output/formatter.rs` | JSON, table, and human-readable output formatting |
| `src/commands/auth.rs` | OAuth 2.0 Device Authorization flow, API key login, status |
| `src/commands/patent.rs` | Patent search and CQL query builder |
| `src/commands/academic.rs` | Academic literature search |
| `src/commands/ops.rs` | Direct EPO OPS API (biblio, claims, family, legal, abstract) |
| `src/commands/config_cmd.rs` | CLI configuration management |

## Command Structure

```
flowleap <command> <subcommand> [flags]
```

All commands support `--output json|table|human`, `--dry-run`, and `--verbose`.

## Config Precedence

CLI flags > environment variables > `~/.config/flowleap/config.toml`

## Authentication

Three methods (checked in order):
1. `--token` flag or `FLOWLEAP_TOKEN` env var
2. `--api-key` flag or `FLOWLEAP_API_KEY` env var
3. Stored credentials in `~/.config/flowleap/credentials.toml`

## API Endpoints

| Endpoint | Method | Auth Required |
|----------|--------|---------------|
| `/oauth/device` | POST | No |
| `/oauth/device/token` | POST | No |
| `/oauth/device/approve` | POST | Yes |
| `/v1/patent-search` | POST | Yes |
| `/v1/build-patent-query` | POST | Yes |
| `/v1/academic-search` | POST | Yes |
| `/v1/ops/biblio?doc={id}` | GET | Yes |
| `/v1/ops/abstract?doc={id}` | GET | Yes |
| `/v1/ops/family?doc={id}` | GET | Yes |
| `/v1/ops/legal?doc={id}` | GET | Yes |
| `/v1/ops/fulltext/claims?doc={id}&lang={lang}` | GET | Yes |
| `/v1/ops/fulltext/description?doc={id}&lang={lang}` | GET | Yes |
| `/api/profile` | GET | Yes |
| `/api/usage` | GET | Yes |

OPS endpoints wrap payloads in a response envelope:

```json
{ "success": true, "data": { /* endpoint-specific */ }, "cached": false, "executionTimeMs": 432 }
```

On failure:

```json
{ "success": false, "error": "...", "code": "NOT_FOUND", "status": 404 }
```

Error `code` values: `MISSING_PARAM` (400), `NOT_FOUND` (404), `RATE_LIMITED` (429), `INTERNAL_ERROR` (500). Canonical endpoint list: the backend's `GET /v1/ops/health` response. Live OpenAPI spec: `<base-url>/docs/json` (requires `ENABLE_SWAGGER=true` in production).

## Security

- Never output stored credentials (API keys, tokens) in logs or verbose mode
- Use `--dry-run` for safety when testing mutating operations
- Authorization header is stripped from verbose output

## Skills

The `skills/` directory contains SKILL.md files for AI agent consumption. Each skill describes one CLI capability with usage examples, flags, and expected output. Skills are organized into:

- **Service skills** (`flowleap-*`): One per CLI command
- **Persona skills** (`persona-*`): Role-based bundles for specific workflows
- **Recipe skills** (`recipe-*`): Multi-step workflow templates

## Environment Variables

| Variable | Description |
|----------|-------------|
| `FLOWLEAP_API_KEY` | API key for authentication |
| `FLOWLEAP_TOKEN` | Bearer token for authentication |
| `FLOWLEAP_BASE_URL` | API base URL override |


## Testing

- Unit tests: config parsing, credential storage, output formatting
- Integration tests: in `tests/` directory
- Test with `cargo test`
