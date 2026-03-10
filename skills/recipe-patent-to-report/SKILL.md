---
name: recipe-patent-to-report
version: 1.0.0
description: "Recipe: Extract all data from a patent for structured analysis."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-ops", "flowleap-patent"]
---

# Recipe: Patent to Report

Extract all data from a patent document for structured analysis.

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

### Step 2: Find Related Patents

```bash
# Search for related patents using key terms from the abstract
flowleap patent search --query "<key terms from abstract>" --limit 10 --output json
```

## Output

Complete patent data package including:
- Bibliographic data (title, applicant, dates, classification)
- Abstract and description text
- Full claims text
- Patent family members across jurisdictions
- Legal status (active, expired, pending)
- Related patents in the same field
