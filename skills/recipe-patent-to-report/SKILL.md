---
name: recipe-patent-to-report
description: Recipe for extracting everything about one patent into a structured report — bibliography, abstract, claims, description, family, legal status, prosecution timeline, figures, and related art. Trigger when the user asks for a complete profile, dossier, or report on a specific patent.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-ops", "flowleap-patent"]
---

# Recipe: Patent to Report

Extract all data from a patent document for structured analysis.

## Steps

### Step 1: One-Call Snapshot

```bash
flowleap --json summary <patent-number>    # biblio + legal status + family + term
flowleap --json timeline <patent-number>   # chronological prosecution events
```

### Step 2: Gather Full Text

```bash
flowleap --json ops abstract <patent-number>
flowleap --json ops claims <patent-number>
flowleap --json ops description <patent-number>
```

### Step 3: Figures

```bash
flowleap figures <patent-number>                          # figure metadata
flowleap figures <patent-number> --out figure.png --page 3  # save one page
```

### Step 4: Find Related Patents

```bash
# Search for related patents using key terms from the abstract
flowleap --json patent search --query "<key terms from abstract>" --limit 10
```

## Output

Complete patent data package including:
- Bibliographic data (title, applicant, dates, classification)
- Legal status, family members, and estimated term
- Prosecution timeline (register + INPADOC legal events)
- Abstract, description, and full claims text
- Drawings/figures
- Related patents in the same field
