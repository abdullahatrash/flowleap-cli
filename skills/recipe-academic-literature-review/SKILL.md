---
name: recipe-academic-literature-review
version: 1.0.0
description: "Recipe: Academic literature review combined with patent analysis."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-academic", "flowleap-patent", "flowleap-chat"]
---

# Recipe: Academic Literature Review

Combine academic and patent searches for a comprehensive technology review.

## Steps

### Step 1: Academic Search

```bash
flowleap academic search "<research topic>" --limit 20 --output json
```

### Step 2: Patent Search for the Same Topic

```bash
flowleap patent build-query "<research topic>"
flowleap patent search --query "<generated CQL>" --limit 20 --output json
```

### Step 3: OCR Legacy Documents (if needed)

```bash
flowleap ocr extract paper.pdf --format markdown
```

### Step 4: AI Literature Review

```bash
flowleap chat --system "You are a research scientist writing a literature review." \
  "Write a structured literature review on <topic> based on the following sources:
   Academic papers: <list papers with titles and authors>
   Related patents: <list patents with titles>

   Structure:
   1. Introduction and scope
   2. Key research themes and findings
   3. Patent activity and commercial applications
   4. Gaps between academic research and patented technology
   5. Future research directions
   6. References"
```

## Output

A structured literature review covering both academic and patent landscapes.
