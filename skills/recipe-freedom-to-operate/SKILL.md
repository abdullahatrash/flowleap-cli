---
name: recipe-freedom-to-operate
version: 1.0.0
description: "Recipe: Freedom-to-operate search for a product or technology."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
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
flowleap patent search --query "<CQL for feature 1>" --source epo --limit 20 --output json
flowleap patent search --query "<CQL for feature 1>" --source uspto --limit 20 --output json
# Repeat for other features
```

### Step 3: Check Legal Status of Relevant Patents

For each potentially blocking patent:

```bash
flowleap ops legal <patent-number> --output json    # Is it active?
flowleap ops family <patent-number> --output json   # Where is it filed?
flowleap ops claims <patent-number> --output json   # What does it cover?
```

## Output

FTO data package per blocking patent:
- Patent search results per product feature
- Legal status (active, expired, abandoned)
- Geographic coverage (family members)
- Claims text for scope assessment
