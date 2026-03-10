---
name: recipe-claim-analysis
version: 1.0.0
description: "Recipe: Analyze patent claims for scope, dependencies, and infringement risk."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-ops", "flowleap-chat"]
---

# Recipe: Claim Analysis

Analyze patent claims to understand scope, dependencies, and potential infringement risks.

## Steps

### Step 1: Extract Claims

```bash
flowleap ops claims <patent-number> --output json
```

### Step 2: Get Context

```bash
flowleap ops abstract <patent-number>
flowleap ops biblio <patent-number>
```

### Step 3: AI Claim Analysis

```bash
flowleap chat --system "You are a patent attorney specializing in claim interpretation. Analyze claims systematically." \
  "Analyze the claims of patent <number>:
   1. Identify each independent claim and its scope
   2. Map dependent claims to their parent claims
   3. Identify the broadest independent claim
   4. List the key limitations in each independent claim
   5. Identify potential design-around opportunities
   6. Assess claim clarity and potential indefiniteness issues"
```

### Step 4: Compare with Product/Process (Optional)

```bash
flowleap chat --system "You are a patent attorney assessing infringement risk." \
  "Compare the claims of patent <number> against the following product/process description: <description>. For each independent claim, assess whether each limitation is met literally or under doctrine of equivalents."
```

## Output

A structured claim analysis with scope assessment, dependency map, and optional infringement comparison.
