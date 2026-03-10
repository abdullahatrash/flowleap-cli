---
name: recipe-academic-literature-review
version: 1.0.0
description: "Recipe: Academic literature review combined with patent analysis."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-academic", "flowleap-patent", "flowleap-ocr"]
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

## Output

Combined dataset of:
- Academic papers (title, authors, year, source)
- Related patents (publication number, title, applicant, date)
- OCR-extracted text from legacy documents
