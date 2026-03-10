---
name: recipe-freedom-to-operate
version: 1.0.0
description: "Recipe: Freedom-to-operate analysis for a product or technology."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-patent", "flowleap-ops", "flowleap-chat"]
---

# Recipe: Freedom-to-Operate (FTO) Analysis

Assess whether a product or technology may infringe existing patents.

## Steps

### Step 1: Define the Product/Technology

Describe the key technical features of your product or technology.

### Step 2: Generate Targeted Searches

```bash
# Generate CQL for each key feature
flowleap patent build-query "<feature 1 description>"
flowleap patent build-query "<feature 2 description>"
flowleap patent build-query "<feature 3 description>"
```

### Step 3: Search for Potentially Blocking Patents

```bash
# Search both databases for each feature
flowleap patent search --query "<CQL for feature 1>" --source epo --limit 20
flowleap patent search --query "<CQL for feature 1>" --source uspto --limit 20
# Repeat for other features
```

### Step 4: Check Legal Status of Relevant Patents

For each potentially blocking patent:

```bash
flowleap ops legal <patent-number>    # Is it active?
flowleap ops family <patent-number>   # Where is it filed?
flowleap ops claims <patent-number>   # What does it actually cover?
```

### Step 5: AI FTO Assessment

```bash
flowleap chat --system "You are a patent attorney conducting a freedom-to-operate analysis. Be thorough and identify risks." \
  "Conduct an FTO assessment for the following product: <product description>.
   Potentially blocking patents found: <list patents with claim summaries>.
   For each patent:
   1. Is it still active? (legal status)
   2. Does it cover our target markets? (family/geography)
   3. Do our product features fall within the claim scope?
   4. What is the infringement risk level (high/medium/low)?
   5. What design-around options exist?"
```

## Output

An FTO risk matrix with risk levels per patent and recommended design-around strategies.
