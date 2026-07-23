---
name: flowleap-patstat
description: Portfolio Analytics over the PATSTAT snapshot — structured-criteria aggregation by named applicant, CPC/IPC class, office, year, family, and grant status, with harmonized entity resolution and Data Edition provenance. Trigger when an agent needs a named applicant's filing portfolio, structured-criteria corpus counts (not free-text search), or any number that must carry a PATSTAT edition citation.
---

# FlowLeap Patstat (Portfolio Analytics)

Auth and global flags: see `flowleap-shared`.

## Topic Analytics vs Portfolio Analytics — routing rule

FlowLeap runs two aggregate-analytics engines, split by *criteria shape*, not
by metric:

- **Topic Analytics** (`flowleap analytics`, the Google-Patents corpus
  engine) — the question's essential criterion is **free-text keywords** over
  title/abstract ("quantum computing filings over time"). Publication-level
  counts, substring name matching, per-query cost.
- **Portfolio Analytics** (`flowleap patstat`, this skill, the PATSTAT
  engine) — the question is expressible in **structured criteria**: named
  applicant (entity-resolved, harmonized names), CPC/IPC class, office, year,
  family, grant status. Family-level counting, zero marginal cost.

Routing rule: if the question needs free text, use `flowleap analytics`; if
it is structured criteria — especially a named company — use
`flowleap patstat`. Individual documents (one known publication or
application) are neither — use the search/retrieval skills (`flowleap-patent`,
`flowleap-uspto`, `flowleap-ops`).

## Portfolio

```
flowleap --json patstat portfolio "<applicant name>" --from-year 2015 --to-year 2023
```

(The `patstat` command family lands with cli issue #32; syntax is per that
spec. This fence is left unlabeled — not `bash` — until the command is wired,
so it is not picked up by this repo's documented-example validator; flip it
to a `bash` fence once `flowleap patstat` exists in the CLI you're running
against.)

Response shape: a quotable `summary` line first — relay it verbatim before
adding any narrative — then filings-by-year/office/grant-status aggregate
tables, then a `data_edition` provenance line.

## Ambiguous applicant (422)

An unresolved applicant name returns HTTP 422 with a candidate list. This is
an **interaction step, not a retryable error**: render every candidate to the
user in both `--json` and human output, and **never auto-pick one**. Once the
user picks, re-run with the exact candidate name and pin that exact string —
a caller that needs to repeat the query (e.g. a `recipe-custom-dashboard`
script) hard-codes the resolved name as a constant so the choice is made once,
not re-asked on every run.

## Data Edition

PATSTAT is published in discrete snapshot editions (~twice a year). Every
Portfolio Analytics answer carries its `data_edition` — treat Portfolio
Analytics as a snapshot with a name, not live data. Two answers are only
comparable within the **same** `data_edition`; always surface the edition
alongside any number quoted from this skill.

## patstat_unavailable

If the backend has no PATSTAT database configured, it returns a
`patstat_unavailable` error. Say so plainly ("backend has no PATSTAT dataset
configured") and stop — this is a deployment gap, not a transient failure; do
not retry.

Also available as `flowleap tools run patstat_portfolio …` once the backend
tool-registry entry lands — see `flowleap-tools`.

```bash
flowleap --json tools run patstat_portfolio applicant="<applicant name>"
```
