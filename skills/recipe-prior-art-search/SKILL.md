---
name: recipe-prior-art-search
description: Comprehensive prior-art / novelty search before filing — natural-language query generation, dual EPO/USPTO patent search, an academic literature sweep, and X/Y/A tagging of the closest hits. Trigger when the user asks to find prior art for an invention, run a novelty search, or check what already exists before filing.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-patent", "flowleap-uspto", "flowleap-academic", "flowleap-ops"]
---

# Recipe: Prior Art Search

A multi-step workflow for comprehensive prior-art searching. Each database uses
its own query syntax — see `flowleap-uspto` for the USPTO Lucene grammar.

## Steps

### Step 1: Generate Search Queries

```bash
# --focus broad widens recall for a first novelty pass
flowleap patent build-query "<describe the invention in natural language>" --focus broad
flowleap uspto build-query "<describe the invention in natural language>" --focus broad
```

Done when you have one EPO CQL query and one USPTO ODP query for the invention.

### Step 2: Search Patents (EPO + USPTO)

```bash
flowleap --json patent search --query "<CQL from step 1>" --limit 20
flowleap --json uspto search --query "<recommended_query from step 1>" --limit 20
```

Run **both** — they are different databases, not interchangeable:

- `patent search` hits EPO OPS: worldwide DOCDB coverage, including EPO's own
  bibliographic copy of US documents. `--countries US` narrows this EPO
  collection to US members; it does **not** reach USPTO's Open Data Portal.
- `uspto search` hits USPTO ODP directly, returning US application and
  prosecution metadata (the `patentFileWrapperDataBag`) that the EPO copy lacks.

So for US prior art, the USPTO leg must go through `uspto build-query` →
`uspto search`. Do not substitute `patent search --countries US` for it — that
returns EPO's bibliographic copy of US documents, not USPTO ODP records.

Done when both databases have returned ranked results.

### Step 3: Search Academic Literature

```bash
flowleap --json academic search "<invention keywords>" --limit 15
flowleap --json npl "<invention keywords>" --limit 10
```

### Step 4: Deep Dive on the Closest Hits

Deep-dive every hit whose abstract maps to at least one independent feature of
the invention — at minimum the top 5 by rank:

```bash
flowleap ops abstract <patent-number>
flowleap ops claims <patent-number>
flowleap ops family <patent-number>
```

Done when each qualifying hit has its claims and family pulled.

### Step 5: Tag Each Reference X / Y / A

Against the invention's features, tag every retained reference:
- **X** — alone anticipates a feature (novelty-destroying)
- **Y** — anticipates only in combination with another reference
- **A** — general background

Done when every retained reference carries an X/Y/A tag, X-tagged first.

## Output

A prior-art table with:
- One row per patent **family** (the closest member represents the family; use
  `ops family` to collapse duplicates)
- Each row tagged X / Y / A, X-tagged references surfaced first
- Academic papers on the same topic
- Claims and abstracts from the closest prior art
