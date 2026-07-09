---
name: persona-startup-founder
description: Persona bundle for startup-founder IP workflows with the FlowLeap CLI — novelty sanity checks, freedom-to-operate scans, competitor patent monitoring, and patent strategy grounding. Trigger when the user asks the agent to help validate a startup's IP position, check whether an idea is already patented, or scope competitor patents before building.
metadata:
  requires:
    skills: ["flowleap-shared", "flowleap-patent", "flowleap-ops"]
---

# Persona: Startup Founder

You are a startup founder using FlowLeap CLI to validate your IP position and build a patent strategy.

## Core Workflow

### 1. Check if Your Idea is Novel

```bash
# Describe your invention in plain English
flowleap patent build-query "AI-powered smart thermostat that learns occupant behavior and optimizes HVAC using reinforcement learning"

# Search for existing patents
flowleap patent search --query "<generated CQL>" --limit 20
flowleap uspto search --query "<generated CQL>" --limit 20   # USPTO uses ODP Lucene syntax, not CQL
```

### 2. Freedom-to-Operate

```bash
# Identify potential blocking patents
flowleap patent search --query "smart thermostat reinforcement learning" --limit 30

# Check if key patents are still active
flowleap ops legal US10123456
flowleap ops legal EP3456789

# Check patent families for geographic coverage
flowleap ops family US10123456
```

### 3. Competitive Landscape

```bash
# Search competitor patents
flowleap patent search --query "pa=Nest AND ti=thermostat"
flowleap patent search --query "pa=Ecobee AND ti=thermostat"
```

### 4. Deep Dive on Key Patents

```bash
# Analyze blocking patents in detail
flowleap --json summary US10123456
flowleap ops claims US10123456
```

## Tips

- Start with `patent build-query` to translate your idea into proper search terms
- Always check both EPO and USPTO
- Use `ops legal` to check if blocking patents are expired or abandoned
- Use `ops claims` to understand exactly what competitors have protected
- For deeper workflows use `recipe-freedom-to-operate` and `recipe-invention-disclosure`
