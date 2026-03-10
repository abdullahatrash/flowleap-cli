---
name: recipe-patent-landscape
version: 1.0.0
description: "Recipe: Patent landscape analysis for a technology area."
metadata:
  category: "recipe"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-patent", "flowleap-ops"]
---

# Recipe: Patent Landscape Analysis

Map the patent landscape for a technology area, identifying key players and trends.

## Steps

### Step 1: Define Search Scope

```bash
# Generate CQL for the technology area
flowleap patent build-query "<technology description>"
```

### Step 2: Broad Patent Search

```bash
# Search both databases
flowleap patent search --query "<CQL>" --source epo --limit 50 --output json
flowleap patent search --query "<CQL>" --source uspto --limit 50 --output json
```

### Step 3: Identify Key Players

```bash
# Search top applicants individually
flowleap patent search --query "pa=<company1> AND ti=<technology>" --output json
flowleap patent search --query "pa=<company2> AND ti=<technology>" --output json
```

### Step 4: Check Recent Activity

```bash
# Recent filings using CQL date filters
flowleap ops search --cql "ti=<technology> AND pd>=2023" --start 1 --end 50
```

## Output

A dataset of patent results segmented by database, applicant, and filing date for landscape analysis.
