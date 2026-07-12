---
name: flowleap-uspto
description: Search USPTO Open Data Portal records with Lucene queries, fetch granted patents, applications and continuity chains, and build ODP queries from natural language. Trigger when an agent needs US application/prosecution metadata, grant lookups, continuity (parent/child) chains, or USPTO-specific searches.
---

# FlowLeap USPTO (Open Data Portal)

Auth and global flags: see `flowleap-shared`.

## Search

ODP uses Lucene syntax over application metadata (get the grammar via
`flowleap --json tools run get_search_syntax provider=uspto`):

```bash
flowleap --json uspto search --query 'applicationMetaData.inventionTitle:"machine learning"' --limit 5
```

Results arrive in `patentFileWrapperDataBag`.

**ODP is title + metadata only — there is no abstract/claims full-text.** The
only free-text field is `applicationMetaData.inventionTitle`. A distinguishing
feature that lives in the abstract (e.g. "UV-C sterilization" on an earbud
charging case titled only "CHARGING CASE FOR EARBUDS") cannot be matched, so
never AND an abstract-only qualifier onto an ODP search. For a recall pass,
search the **core device noun** in the title (with singular/plural variants)
and triage abstracts afterwards with `flowleap ops abstract <number>`:

```bash
flowleap --json uspto search --query 'applicationMetaData.inventionTitle:earbuds AND applicationMetaData.inventionTitle:"charging case"' --limit 25
```

**Zero-recall fallback.** If a search returns nothing, the CLI does not hand
back a silent empty set: when the query carries a `cpcClassificationBag:`
constraint it strips that filter and retries once (a mis-guessed CPC class is a
common cause of zero recall), then, if still empty, prints guidance to broaden
to a title search. Watch stderr for these notes.

## Search with a full request body

`uspto search` accepts a complete ODP request body via `--body` (inline JSON,
or `-` for stdin) or `--body-file` — this is how you submit the object that
`uspto build-query` generates (see below). `--query` and `--body`/`--body-file`
are mutually exclusive.

```bash
flowleap --json uspto search --body '{"q":"applicationMetaData.inventionTitle:\"machine learning\"","pagination":{"limit":5}}'
flowleap --json uspto search --body-file query.json
```

## Build a query from natural language

```bash
flowleap --json uspto build-query "quantum error correction filed after 2022" --focus precise --allow-external-processing
```

Live query generation sends the description to FlowLeap and then to Anthropic
or OpenAI. `--allow-external-processing` records explicit consent; use
`--dry-run --dry-run-redacted` to inspect only the request shape locally.

Returns `strategy.recommended_query` — a **complete ODP request body** (not a
string). Submit it directly with `--body`:

```bash
flowleap --json uspto search --body '<recommended_query JSON>'
```

`--focus` is one of `broad` | `precise` | `comprehensive`. Note that the CPC
class in a generated query is a heuristic guess and can be wrong; if a run
returns 0, rely on the zero-recall fallback above or a title recall pass rather
than trusting the guessed class.

## Lookups

```bash
flowleap --json uspto grant 11800000              # granted patent by number
flowleap --json uspto application 16123456        # application by number
flowleap --json uspto continuity 16123456         # parent/child chain
```

Continuity is also available as `flowleap tools run get_continuity application_number=16123456`.
