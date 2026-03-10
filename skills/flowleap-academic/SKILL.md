---
name: flowleap-academic
version: 1.0.0
description: "FlowLeap Academic: Search academic literature."
metadata:
  category: "patent-ai"
  requires:
    bins: ["flowleap"]
  cliHelp: "flowleap academic --help"
---

# FlowLeap Academic

Prerequisite: Read `flowleap-shared` for authentication and global flags.

## Usage

```bash
flowleap academic search <query> [flags]
```

Posts to `/v1/academic-search`. Returns academic papers with title, authors, year, and source.

## Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--limit` | Maximum results | `10` |

## Examples

```bash
# Basic search
flowleap academic search "machine learning patent classification"

# With limit
flowleap academic search "CRISPR gene editing applications" --limit 20

# JSON output for agents
flowleap academic search "neural network optimization" --output json
```

## Response Format (JSON)

```json
{
  "results": [
    {
      "title": "Machine Learning in Patent Analysis",
      "authors": "Smith, J.; Doe, A.",
      "year": 2024,
      "source": "Nature AI"
    }
  ]
}
```
