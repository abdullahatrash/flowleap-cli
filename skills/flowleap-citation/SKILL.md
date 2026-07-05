---
name: flowleap-citation
description: USPTO enriched citation data from office actions — citations by application, forward citations of a document, citation statistics and X-category novelty-destroying references. Trigger when an agent assesses novelty risk, examiner-cited prior art, or how often a patent is cited against later applications.
---

# FlowLeap Citation Search (USPTO enriched citations)

## By application

```bash
flowleap --json citation search 16123456 --size 20
flowleap --json citation search 16123456 --category x --examiner-cited-only
```

## Forward citations (who cites this document)

```bash
flowleap --json citation forward US10123456 --size 20
```

## Analysis shortcuts

```bash
flowleap --json citation stats 16123456      # counts by category/source
flowleap --json citation novelty 16123456    # X-rated novelty-destroying citations
```

Categories: `x` (novelty-destroying), `y` (inventive-step with combination),
`a` (background), `all`. Also available as tools:
`search_office_action_citations` (by application) and
`search_enriched_citations` (forward, by cited document).
