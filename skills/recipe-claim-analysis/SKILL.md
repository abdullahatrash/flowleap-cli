---
name: recipe-claim-analysis
version: 1.0.0
description: "Recipe: Extract and analyze patent claims with full context."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-ops"]
---

# Recipe: Claim Analysis

Extract patent claims with full context for detailed analysis.

## Steps

### Step 1: Extract Claims

```bash
flowleap ops claims <patent-number> --output json
```

### Step 2: Get Context

```bash
flowleap ops abstract <patent-number> --output json
flowleap ops biblio <patent-number> --output json
flowleap ops description <patent-number> --output json
```

### Step 3: Check Related Patents

```bash
flowleap ops family <patent-number> --output json
```

## Output

Complete claim data with supporting context:
- Full claims text (independent and dependent)
- Abstract for technical field context
- Bibliographic data for filing details
- Description for claim interpretation
- Family members for jurisdiction coverage
