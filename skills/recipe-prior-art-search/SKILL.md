---
name: recipe-prior-art-search
description: Recipe for a comprehensive prior art search — natural-language query generation, dual EPO/USPTO patent search, academic literature sweep, and deep dives on the closest hits. Trigger when the user asks to find prior art for an invention, run a novelty search, or check what already exists before filing.
metadata:
  requires:
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
flowleap --json patent search --query "<CQL from step 1>" --limit 20
flowleap --json uspto search --query "<CQL from step 1>" --limit 20   # USPTO uses ODP Lucene syntax, not CQL
```

### Step 3: Search Academic Literature

```bash
flowleap --json academic search "<invention keywords>" --limit 15
flowleap --json npl "<invention keywords>" --limit 10
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
