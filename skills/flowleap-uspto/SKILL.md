---
name: flowleap-uspto
description: Search USPTO Open Data Portal records with Lucene queries, fetch granted patents, applications and continuity chains, and build ODP queries from natural language. Trigger when an agent needs US application/prosecution metadata, grant lookups, continuity (parent/child) chains, or USPTO-specific searches.
---

# FlowLeap USPTO (Open Data Portal)

## Search

ODP uses Lucene syntax over application metadata (get the grammar via
`flowleap --json tools run get_search_syntax provider=uspto`):

```bash
flowleap --json uspto search --query 'applicationMetaData.inventionTitle:"machine learning"' --limit 5
```

Results arrive in `patentFileWrapperDataBag`.

## Build a query from natural language

```bash
flowleap --json uspto build-query "quantum error correction filed after 2022" --focus precise
```

Returns `strategy.recommended_query` — a **complete ODP request body** (not a
string). Submit it directly:

```bash
flowleap --json api request post /v1/patent-search-uspto/search --body '<recommended_query JSON>'
```

`--focus` is one of `broad` | `precise` | `comprehensive`.

## Lookups

```bash
flowleap --json uspto grant 11800000              # granted patent by number
flowleap --json uspto application 16123456        # application by number
flowleap --json uspto continuity 16123456         # parent/child chain
```

Continuity is also available as `flowleap tools run get_continuity application_number=16123456`.
