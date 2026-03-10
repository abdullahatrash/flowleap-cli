---
name: flowleap-patent
version: 1.0.0
description: "FlowLeap Patent: Search patents and build CQL queries."
metadata:
  category: "patent-ai"
  requires:
    bins: ["flowleap"]
  cliHelp: "flowleap patent --help"
---

# FlowLeap Patent

Prerequisite: Read `flowleap-shared` for authentication and global flags.

## Commands

### Search Patents

```bash
flowleap patent search --query <query> [flags]
```

Posts to `/v1/patent-search`. Returns patent results with publication number, title, applicant, and date.

| Flag | Description | Default |
|------|-------------|---------|
| `--query`, `-q` | Search query (required) | — |
| `--source` | Database: `epo`, `uspto` | `epo` |
| `--limit` | Maximum results | `10` |

#### Examples

```bash
# Basic search
flowleap patent search --query "solar panel efficiency"

# USPTO source with limit
flowleap patent search --query "lithium battery" --source uspto --limit 20

# JSON output for agents
flowleap patent search --query "CRISPR gene editing" --output json
```

#### Response Format (JSON)

```json
{
  "results": [
    {
      "publicationNumber": "EP1234567",
      "title": "Solar Panel with Improved Efficiency",
      "applicant": "SolarCorp Inc.",
      "publicationDate": "2024-01-15"
    }
  ]
}
```

### Build CQL Query

```bash
flowleap patent build-query <description> [flags]
```

Posts to `/v1/build-patent-query`. Converts natural language to CQL (Common Query Language) for EPO patent searches.

| Flag | Description | Default |
|------|-------------|---------|
| `--model` | AI model for query generation | — |

#### Examples

```bash
# Natural language to CQL
flowleap patent build-query "patents about lithium battery recycling filed by Tesla"

# With specific model
flowleap patent build-query --model patent-claude-sonnet "renewable energy storage systems"

# JSON output
flowleap patent build-query --output json "autonomous vehicle lidar sensors"
```

#### Response Format (JSON)

```json
{
  "query": "ti=lithium AND ti=battery AND ti=recycling AND pa=Tesla",
  "explanation": "Searches for patents with lithium, battery, and recycling in the title filed by Tesla."
}
```

## Workflow: Natural Language to Patent Results

1. Build a CQL query: `flowleap patent build-query "your description"`
2. Use the generated CQL in a search: `flowleap patent search --query "<CQL>"`
