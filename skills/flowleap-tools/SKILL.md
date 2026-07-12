---
name: flowleap-tools
description: Discover and run FlowLeap backend tools through the agent-first /v1/tools facade — one stable contract (search_patents, get_bibliography, get_claims, get_patent_summary, compare_patents, reference_search, …) instead of provider-specific endpoints. Trigger when an agent needs patent data and wants a uniform tool interface, needs to discover what the backend can do, or wants compound operations like patent summaries, comparisons, or prosecution timelines.
---

# FlowLeap Tools (agent-first facade)

`flowleap tools` is the uniform interface to every backend capability. Prefer it
over the provider-specific commands when you want stable tool names and JSON
schemas you can discover at runtime.

## Discover

```bash
flowleap --json tools list                     # all tools + descriptions
flowleap --json tools describe get_bibliography  # JSON input schema for one tool
flowleap --json tools openapi                  # full OpenAPI document
```

## Run

Inputs are JSON objects. Three equivalent styles — `key=value` pairs are easiest
for flat inputs; values parse as JSON when possible (numbers, booleans, arrays):

```bash
flowleap --json tools run get_bibliography patent_number=EP1000000
flowleap --json tools run search_patents --input '{"query":"ti=\"solid state battery\"","range":"1-10"}'
flowleap --json tools run compare_patents --input '{"patent_numbers":["EP1000000","US10123456"]}'
```

Output is the tool's `data` payload as JSON. Errors print an envelope with
`status` and `body.error.code` (`UNKNOWN_TOOL` 404, `INVALID_INPUT` 422 with
`issues[]`, `NOT_FOUND` 404, `RATE_LIMITED` 429 with `retryAfterSeconds`).

## Tool inventory

Search: `search_patents` (provider=epo_ops CQL | uspto Lucene), `get_search_syntax`,
`search_uspto_portfolio_by_customer_number`.

Retrieval (any publication number, EPO OPS worldwide): `get_bibliography`,
`get_abstract`, `get_claims`, `get_description`, `get_fulltext`,
`get_patent_family`, `get_legal_status`, `get_register_events`, `get_citations`,
`get_patent_image`, `convert_patent_number`.

USPTO prosecution: `get_continuity`, `search_office_action_citations`,
`search_enriched_citations`.

Compound (one call, multiple sources): `get_patent_summary`, `compare_patents`
(2-10 patents), `get_prosecution_timeline`, `get_patent_term`.

Reference: `reference_search` (patent-law RAG: EPC, EPO Guidelines, MPEP, …).
Meta: `server_info`.

## Recipes

Patent snapshot in one call:

```bash
flowleap --json tools run get_patent_summary patent_number=EP1000000
```

Portfolio triage:

```bash
flowleap --json tools run search_uspto_portfolio_by_customer_number customer_number=23456 limit=50
```

Query building still lives in the provider commands:

```bash
flowleap --json patent build-query "solid state battery separators" --focus precise --allow-external-processing
flowleap --json tools run search_patents --input "{\"query\": \"<recommended_cql>\"}"
```

## Auth

Auth, subscription, and rate limits: see `flowleap-shared`.
