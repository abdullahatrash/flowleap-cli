---
name: recipe-patent-to-report
version: 1.0.0
description: "Recipe: Generate a structured report from a patent document."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-ops", "flowleap-chat"]
---

# Recipe: Patent to Report

Extract all data from a patent and generate a structured analysis report.

## Steps

### Step 1: Gather Patent Data

```bash
flowleap ops biblio <patent-number> --output json
flowleap ops abstract <patent-number> --output json
flowleap ops claims <patent-number> --output json
flowleap ops description <patent-number> --output json
flowleap ops family <patent-number> --output json
flowleap ops legal <patent-number> --output json
```

### Step 2: Generate Report

```bash
flowleap chat --system "You are a patent analyst. Generate a structured report from the following patent data." \
  "Generate a comprehensive report for patent <number> covering:
   1. Title and bibliographic summary
   2. Abstract and technical field
   3. Independent claim analysis (scope, key limitations)
   4. Dependent claim mapping
   5. Patent family coverage (jurisdictions)
   6. Legal status (active, expired, pending)
   7. Commercial relevance assessment"
```

### Step 3: Find Related Patents

```bash
# Search for related patents using key terms from the abstract
flowleap patent search --query "<key terms from abstract>" --limit 10
```

## Output

A structured patent analysis report suitable for IP due diligence or portfolio review.
