---
name: flowleap-patent
description: Search EPO patents with CQL and build CQL queries from natural language through the FlowLeap backend. Trigger when an agent needs European or worldwide patent search results, needs a plain-English invention description turned into a CQL query, or wants to tune query strategy (broad, precise, comprehensive). For US-specific ODP searches see flowleap-uspto.
---

# FlowLeap Patent

Auth and global flags: see `flowleap-shared`.

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
flowleap patent search --query "CRISPR gene editing" --json
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
flowleap patent build-query --json "autonomous vehicle lidar sensors"
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
