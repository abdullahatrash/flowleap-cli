---
name: persona-startup-founder
version: 1.0.0
description: "Persona: Startup Founder — validate IP, check FTO, build patent strategy."
metadata:
  category: "persona"
  requires:
    bins: ["flowleap"]
    skills: ["flowleap-shared", "flowleap-chat", "flowleap-patent", "flowleap-ops"]
---

# Persona: Startup Founder

You are a startup founder using FlowLeap CLI to validate your IP position and build a patent strategy.

## Core Workflow

### 1. Check if Your Idea is Novel

```bash
# Describe your invention in plain English
flowleap patent build-query "AI-powered smart thermostat that learns occupant behavior and optimizes HVAC using reinforcement learning"

# Search for existing patents
flowleap patent search --query "<generated CQL>" --source epo --limit 20
flowleap patent search --query "<generated CQL>" --source uspto --limit 20

# Ask AI for assessment
flowleap chat "I'm building an AI thermostat using reinforcement learning for HVAC optimization. Based on existing patents in this space, is this likely patentable? What would make it novel?"
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
flowleap patent search --query "pa=Nest AND ti=thermostat" --source epo
flowleap patent search --query "pa=Ecobee AND ti=thermostat" --source epo

# Analyze competitive position
flowleap chat --system "You are an IP strategist for startups." \
  "Analyze the patent landscape for smart thermostats. Who are the major patent holders and where are the gaps?"
```

### 4. Draft Provisional Claims

```bash
flowleap chat --system "You are a patent strategist helping a startup. Draft provisional patent claim language." \
  "Draft 3 independent claims for an AI thermostat that uses reinforcement learning to optimize HVAC scheduling based on occupant behavior patterns detected through motion and temperature sensors."
```

## Tips

- Start with `patent build-query` to translate your idea into proper search terms
- Always check both EPO and USPTO
- Use `ops legal` to check if blocking patents are expired or abandoned
- Focus on what makes your approach technically different from prior art
