---
name: recipe-patent-landscape
description: Recipe for patent landscape analysis of a technology area — scoped searches, key-player identification, recent-activity checks, and full-corpus filing analytics. Trigger when the user asks to map a technology space, identify who patents in an area, or report filing trends and white space.
metadata:
  requires:
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
flowleap --json patent search --query "<CQL>" --limit 50
flowleap --json uspto search --query "<CQL>" --limit 50   # USPTO uses ODP Lucene syntax, not CQL
```

### Step 3: Corpus Analytics

```bash
# Filing trends by year, country and CPC breakdowns, top assignees
flowleap --json analytics --keyword "<technology>" --date-from 2015-01-01
flowleap --json analytics --cpc <cpc-prefix> --country US --date-from 2020-01-01
```

### Step 4: Identify Key Players

```bash
# Search top applicants individually
flowleap --json patent search --query "pa=<company1> AND ti=<technology>"
flowleap --json patent search --query "pa=<company2> AND ti=<technology>"
```

### Step 5: Check Recent Activity

```bash
# Recent filings using CQL date filters
flowleap ops search --cql "ti=<technology> AND pd>=2024" --start 1 --end 50
```

## Output

A dataset of patent results segmented by database, applicant, and filing date,
plus corpus-level trend charts (filings per year, top assignees, CPC and
country distributions) for landscape analysis.
