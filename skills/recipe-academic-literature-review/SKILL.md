---
name: recipe-academic-literature-review
description: Recipe for a technology review combining academic literature and patent data — scholarly search across Semantic Scholar, arXiv, and OpenAlex plus a matching patent sweep. Trigger when the user asks for a literature review, a state-of-the-art survey, or a comparison of published research against filed patents.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-academic", "flowleap-patent"]
---

# Recipe: Academic Literature Review

Combine academic and patent searches for a comprehensive technology review.

## Steps

### Step 1: Academic Search

```bash
flowleap --json academic search "<research topic>" --limit 20
flowleap --json academic search "<research topic>" --source arxiv --from-year 2020
```

### Step 2: Widen to OpenAlex NPL

```bash
flowleap --json npl "<research topic>" --from-year 2020 --limit 10
```

### Step 3: Patent Search for the Same Topic

```bash
flowleap patent build-query "<research topic>"
flowleap --json patent search --query "<generated CQL>" --limit 20
```

## Output

Combined dataset of:
- Academic papers (title, authors, year, source, citation counts)
- Related patents (publication number, title, applicant, date)
- The gap between what is published and what is protected
