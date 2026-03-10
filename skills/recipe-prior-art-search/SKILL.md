---
name: recipe-prior-art-search
version: 1.0.0
description: "Recipe: Comprehensive prior art search across patents and academic literature."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-patent", "flowleap-academic", "flowleap-ops"]
---

# Recipe: Prior Art Search

A multi-step workflow for comprehensive prior art searching.

## Steps

### Step 1: Generate Search Queries

```bash
flowleap patent build-query "<describe the invention in natural language>"
```

Take note of the generated CQL query.

### Step 2: Search Patents (EPO + USPTO)

```bash
flowleap patent search --query "<CQL from step 1>" --source epo --limit 20 --output json
flowleap patent search --query "<CQL from step 1>" --source uspto --limit 20 --output json
```

### Step 3: Search Academic Literature

```bash
flowleap academic search "<invention keywords>" --limit 15 --output json
```

### Step 4: Deep Dive on Relevant Results

For each relevant patent found:

```bash
flowleap ops abstract <patent-number>
flowleap ops claims <patent-number>
flowleap ops family <patent-number>
```

## Output

A collection of prior art with:
- Patent results from EPO and USPTO
- Academic papers on the same topic
- Detailed claims and abstracts from the closest prior art
