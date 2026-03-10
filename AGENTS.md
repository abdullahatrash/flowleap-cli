# FlowLeap CLI — Agent & Contributor Guide

## Overview

`flowleap` is a Rust CLI for the FlowLeap Patent AI backend API. It provides patent search, CQL query building, OCR, academic search, and direct EPO OPS access — designed for both human users and AI agents.

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
| `src/commands/ocr.rs` | Multipart file upload OCR |
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
| `/v1/ocr` | POST | Yes |
| `/api/profile` | GET | Yes |
| `/api/usage` | GET | Yes |

## Security

- Never output stored credentials (API keys, tokens) in logs or verbose mode
- Validate file paths before upload (OCR command)
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
