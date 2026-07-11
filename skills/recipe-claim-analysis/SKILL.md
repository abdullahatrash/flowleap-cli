---
name: recipe-claim-analysis
description: Decompose a patent's claims into elements, keywords, and search queries with full supporting context — claims text, abstract, bibliography, description, and family. Trigger when the user asks to analyze what a patent claims, break claims into elements, or interpret claim scope against its specification.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-ops"]
---

# Recipe: Claim Analysis

Extract a patent's claims with full context, then decompose them into elements.

## Steps

### Step 1: Extract Claims

```bash
flowleap --json ops claims <patent-number>
```

Done when you have the full claim set, independent claims identified.

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

### Step 4: Decompose Every Independent Claim

Take each independent claim from Step 1, save its text to a file, and let the
backend break it into elements, keywords, and suggested search queries in one
pass (`--focus full` = elements + search):

```bash
flowleap analyze-claim --file claim1.txt --focus full
```

Repeat for every independent claim. For each dependent claim, note how it
narrows its parent (the added element). Done when every independent claim has an
element breakdown and each dependent claim's added limitation is recorded.

## Output

Complete claim data with supporting context:
- Full claims text (independent and dependent)
- Abstract, bibliographic data, and description for interpretation
- Family members for jurisdiction coverage
- Element breakdown and follow-up search queries per independent claim, plus the
  narrowing limitation added by each dependent claim
