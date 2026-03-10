---
name: flowleap-models
version: 1.0.0
description: "FlowLeap Models: List available AI models."
metadata:
  category: "patent-ai"
  requires:
    bins: ["flowleap"]
  cliHelp: "flowleap models --help"
---

# FlowLeap Models

Prerequisite: Read `flowleap-shared` for global flags.

## Usage

```bash
flowleap models [flags]
```

Fetches `/api/models`. Lists available AI models with ID, provider, and name.

## Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--provider` | Filter by provider: `openai`, `anthropic`, `google` | — |

## Examples

```bash
# List all models
flowleap models

# Filter by provider
flowleap models --provider anthropic

# JSON output
flowleap models --output json
```

## Response Format (JSON)

```json
{
  "data": [
    {
      "id": "patent-claude-sonnet",
      "provider": "anthropic",
      "name": "Patent Claude Sonnet"
    },
    {
      "id": "patent-gemini-3-flash",
      "provider": "google",
      "name": "Patent Gemini 3 Flash"
    }
  ]
}
```

## Agent Usage

Run `flowleap models --output json` to discover available models before using `flowleap chat --model <id>`.
