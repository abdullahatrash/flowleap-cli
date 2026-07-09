---
name: recipe-claim-analysis
description: Recipe for extracting and analyzing a patent's claims with full context — claims text, abstract, bibliography, description, family, plus automated element decomposition. Trigger when the user asks to analyze what a patent claims, break claims into elements, or interpret claim scope with supporting context.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-ops"]
---

# Recipe: Claim Analysis

Extract patent claims with full context for detailed analysis.

## Steps

### Step 1: Extract Claims

```bash
flowleap --json ops claims <patent-number>
```

### Step 2: Get Context

```bash
flowleap --json ops abstract <patent-number>
flowleap --json ops biblio <patent-number>
flowleap --json ops description <patent-number>
```

### Step 3: Check Related Patents

```bash
flowleap --json ops family <patent-number>
```

### Step 4: Decompose the Key Claims

Save the claim text to a file (or pipe it) and let the backend break it into
elements, keywords, and suggested search queries:

```bash
flowleap analyze-claim --file claim1.txt --focus elements
flowleap analyze-claim --file claim1.txt --focus search
```

## Output

Complete claim data with supporting context:
- Full claims text (independent and dependent)
- Abstract for technical field context
- Bibliographic data for filing details
- Description for claim interpretation
- Family members for jurisdiction coverage
- Element breakdown and follow-up search queries per analyzed claim
