---
name: flowleap-academic
description: Search academic literature (Semantic Scholar + arXiv) through the FlowLeap backend with per-source and publication-year filters. Trigger when an agent needs scholarly papers as prior art or technical background — topic searches, restricting to arXiv or Semantic Scholar, or bounding results by publication year. For OpenAlex-backed NPL search see flowleap-npl.
---

# FlowLeap Academic

Auth and global flags: see `flowleap-shared`.

## Usage

```bash
flowleap academic search <query> [flags]
```

Posts to `/v1/academic-search`. Returns academic papers with title, authors, year, and source.

## Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--limit` | Maximum results | `10` |
| `--source` | Source to search: `scholar` or `arxiv` (repeat the flag for both) | all sources |
| `--from-year` | Only papers published in or after this year | none |
| `--to-year` | Only papers published in or before this year | none |

## Examples

```bash
# Basic search
flowleap academic search "machine learning patent classification"

# With limit
flowleap academic search "CRISPR gene editing applications" --limit 20

# arXiv only, bounded by publication year
flowleap academic search "transformer attention mechanisms" --source arxiv --from-year 2020 --to-year 2024

# Both sources explicitly
flowleap academic search "solid state electrolyte" --source scholar --source arxiv

# JSON output for agents
flowleap academic search "neural network optimization" --json
```

## Response Format (JSON)

```json
{
  "success": true,
  "query": "machine learning patent classification",
  "total": 1,
  "cached": false,
  "papers": [
    {
      "title": "Machine Learning in Patent Analysis",
      "authors": ["Smith, J.", "Doe, A."],
      "year": "2024",
      "source": "arxiv",
      "url": "https://example.com/paper",
      "abstract": "..."
    }
  ]
}
```
