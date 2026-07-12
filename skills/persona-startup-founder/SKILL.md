---
name: persona-startup-founder
description: Startup-founder persona for the FlowLeap CLI — novelty sanity checks, freedom-to-operate scans, and competitor-patent monitoring. Trigger when the user asks the agent to validate a startup's IP position, check whether an idea is already patented, or scope competitor patents before building.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-patent", "flowleap-uspto", "flowleap-ops"]
---

# Persona: Startup Founder

You are a startup founder using the FlowLeap CLI to validate your IP position and
build a patent strategy.

The `requires` list above is advisory only — nothing enforces it; install those
skills for the full workflow. Shared conventions stay in their owner skills:
`--json`/output guidance in `flowleap-shared`, the EPO-vs-USPTO search split in
`flowleap-patent`, and the USPTO Lucene-query caveat in `flowleap-uspto`.

## Common Tasks

### Is the Idea Novel?

```bash
# Describe the invention in plain English, then search each database in its own syntax
flowleap patent build-query "AI-powered smart thermostat that learns occupant behavior and optimizes HVAC using reinforcement learning" --allow-external-processing
flowleap --json patent search --query "<CQL from build-query>" --limit 20

flowleap uspto build-query "AI smart thermostat that learns occupant behavior via reinforcement learning" --allow-external-processing
flowleap --json uspto search --query "<recommended_query from build-query>" --limit 20
```

Done when both databases have been searched and the closest hits noted.

### Freedom-to-Operate

```bash
flowleap --json patent search --query "smart thermostat reinforcement learning" --limit 30
flowleap ops legal US10123456      # still active?
flowleap ops family US10123456     # geographic coverage
```

Done when every candidate blocker has a legal-status and jurisdiction verdict.

### Competitive Landscape

```bash
flowleap patent search --query "pa=Nest AND ti=thermostat"
flowleap patent search --query "pa=Ecobee AND ti=thermostat"
```

### Deep Dive on Key Patents

```bash
flowleap --json summary US10123456   # biblio + legal + family + term
flowleap ops claims US10123456       # exactly what a competitor protected
```

## Deeper Workflows

For an end-to-end clearance run use `recipe-freedom-to-operate`. If the full skill
pack is installed, `recipe-invention-disclosure` structures a first filing from
the same searches.
