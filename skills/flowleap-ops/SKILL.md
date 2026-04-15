---
name: flowleap-ops
version: 1.0.0
description: "FlowLeap OPS: Direct EPO Open Patent Services API access."
metadata:
  category: "patent-ai"
  requires:
    bins: ["flowleap"]
  cliHelp: "flowleap ops --help"
---

# FlowLeap OPS

Prerequisite: Read `flowleap-shared` for authentication and global flags.

Direct access to the European Patent Office (EPO) Open Patent Services API.

## Commands

### Search

```bash
flowleap ops search --cql <query> [flags]
```

| Flag | Description | Default |
|------|-------------|---------|
| `--cql` | CQL query string (required) | — |
| `--start` | Start position | `1` |
| `--end` | End position | `25` |

### Document Commands

All document commands take a patent document number (e.g., `EP1234567`):

```bash
flowleap ops biblio <doc>                    # Bibliographic data
flowleap ops claims <doc> [--lang en]        # Claims text (defaults to English)
flowleap ops description <doc> [--lang en]   # Full description (defaults to English)
flowleap ops family <doc>                    # Patent family members
flowleap ops legal <doc>                     # Legal status events
flowleap ops abstract <doc>                  # Abstract text
```

Doc IDs are normalized server-side — `ep1.000.000` and `EP1000000` both resolve.

### Response envelope

OPS endpoints return data wrapped in a success/error envelope. The CLI unwraps
`data` automatically so `--output json` prints just the payload. Pass `--verbose`
to see cache status and execution time. Errors use `code` values: `MISSING_PARAM`,
`NOT_FOUND`, `RATE_LIMITED`, `INTERNAL_ERROR`.

## Examples

```bash
# CQL search
flowleap ops search --cql "ti=solar AND pa=Tesla"

# Get bibliographic data
flowleap ops biblio EP1234567

# Get claims in German
flowleap ops claims US10123456 --lang de

# Get family members (JSON for agents)
flowleap ops family EP1234567 --output json

# Search with pagination
flowleap ops search --cql "ti=battery" --start 1 --end 50

# Verbose shows cache status and timing
flowleap ops biblio EP1234567 --verbose
```

## Workflow: Deep Patent Analysis

1. Search: `flowleap ops search --cql "ti=solar AND pa=Tesla"`
2. Get details: `flowleap ops biblio EP1234567`
3. Read claims: `flowleap ops claims EP1234567`
4. Check family: `flowleap ops family EP1234567`
5. Check legal status: `flowleap ops legal EP1234567`
