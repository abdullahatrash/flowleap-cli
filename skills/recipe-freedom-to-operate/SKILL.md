---
name: recipe-freedom-to-operate
description: Recipe for a freedom-to-operate search — per-feature query generation, dual-database blocking-patent search, and legal-status, family, and claims checks on every candidate. Trigger when the user asks whether a product or technology can be launched without infringing, or requests an FTO/clearance search.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-patent", "flowleap-ops"]
---

# Recipe: Freedom-to-Operate (FTO) Search

Search for potentially blocking patents for a product or technology.

## Steps

### Step 1: Generate Targeted Searches

```bash
# Generate CQL for each key feature
flowleap patent build-query "<feature 1 description>"
flowleap patent build-query "<feature 2 description>"
flowleap patent build-query "<feature 3 description>"
```

### Step 2: Search for Potentially Blocking Patents

```bash
# Search both databases for each feature
flowleap --json patent search --query "<CQL for feature 1>" --limit 20
flowleap --json uspto search --query "<CQL for feature 1>" --limit 20   # USPTO uses ODP Lucene syntax, not CQL
# Repeat for other features
```

### Step 3: Check Legal Status of Relevant Patents

For each potentially blocking patent:

```bash
flowleap --json ops legal <patent-number>     # Is it active?
flowleap --json ops family <patent-number>    # Where is it filed?
flowleap --json ops claims <patent-number>    # What does it cover?
```

Or take the one-call snapshot (bibliography + legal status + family + term):

```bash
flowleap --json summary <patent-number>
```

## Output

FTO data package per blocking patent:
- Patent search results per product feature
- Legal status (active, expired, abandoned) and remaining term
- Geographic coverage (family members)
- Claims text for scope assessment

For element-by-element mapping of a live risk, continue with
`recipe-infringement-charting`.
