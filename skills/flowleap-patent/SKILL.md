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
| `--query`, `-q` | EPO CQL query (required) — e.g. `ti="battery separator"` | — |
| `--limit` | Maximum results (1-100) | `10` |
| `--countries` | Country filter, comma-separated (e.g. `EP,WO`) | none |

For US-specific searches use `flowleap uspto search` (ODP Lucene syntax).

#### Examples

```bash
# Basic search
flowleap patent search --query "solar panel efficiency"

# USPTO source with limit
flowleap uspto search --query "lithium battery" --limit 20   # USPTO uses ODP Lucene syntax, not CQL

# JSON output for agents
flowleap patent search --query "CRISPR gene editing" --output json
```

#### Response Format (JSON)

```json
[
  {
    "docId": "EP1234567.A1",
    "title": "Solar Panel with Improved Efficiency",
    "applicants": ["SolarCorp Inc."],
    "publicationDate": "20240115",
    "abstract": "..."
  }
]
```

### Build CQL Query

```bash
flowleap patent build-query <description> [flags]
```

Posts to `/v1/build-patent-query`. Converts natural language to CQL (Common Query Language) for EPO patent searches.
Use `--dry-run` to verify the request shape without calling the backend model.

| Flag | Description | Default |
|------|-------------|---------|
| `--focus` | Strategy: `broad`, `precise`, `comprehensive` | `comprehensive` |

#### Examples

```bash
# Natural language to CQL
flowleap patent build-query "patents about lithium battery recycling filed by Tesla"

# With a strategy focus
flowleap patent build-query "renewable energy storage systems" --focus comprehensive

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
