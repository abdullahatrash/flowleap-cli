---
name: recipe-academic-literature-review
description: Map the gap between published research and filed patents — a scholarly sweep across Semantic Scholar, arXiv, and OpenAlex aligned against a matching patent search, output centered on the published-versus-protected gap rather than a ranked novelty list. Trigger when the user asks for a literature review, a state-of-the-art survey, or a comparison of academic research against patents.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-academic", "flowleap-npl", "flowleap-patent"]
---

# Recipe: Academic Literature Review

Combine academic and patent searches to map what a field has published against
what it has protected.

Prefer `academic` (Semantic Scholar + arXiv) for CS/ML papers and preprints;
prefer `npl` (OpenAlex) for broad cross-disciplinary journal coverage and
open-access filtering.

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
flowleap --json patent search --query "<CQL from build-query>" --limit 20
```

### Step 4: Synthesize the Gap

Align the academic themes against the patent CPC and assignee clusters. Flag
topics heavily published but lightly patented (open R&D space) and topics
heavily patented but lightly published (crowded IP). Done when each major theme
is classified on the published-versus-protected axis.

## Output

- Academic papers (title, authors, year, source, citation counts)
- Related patents (publication number, title, applicant, date, CPC)
- A published-versus-protected map: themes tagged as open R&D space or crowded IP
