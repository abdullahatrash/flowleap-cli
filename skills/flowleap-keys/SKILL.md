---
name: flowleap-keys
description: Manage BYOK patent-provider credentials (EPO OPS consumer key/secret, USPTO ODP API key) for the FlowLeap CLI — check status, validate live, and hand off to a human for the interactive setup wizard. Trigger when a FlowLeap command fails with provider_keys_required or provider_keys_invalid, when patent data calls error about EPO/USPTO credentials, or when the user asks to configure provider keys.
---

# FlowLeap Provider Keys (BYOK)

Patent data flows through provider APIs that may need the USER's own
credentials: EPO OPS (consumer key + secret — always a pair) and USPTO ODP
(single API key). Keys live in `credentials.toml` (0600) and are forwarded
per-request; the CLI never prints them (verbose/dry-run redact).

## Diagnose

```bash
flowleap --json keys list    # what's configured locally (masked)
flowleap --json keys test    # live verdicts: source user|server|none, valid true|false|null
flowleap --json doctor       # includes a providerKeys section
```

`keys test` needing nothing locally is fine when `source` is `server` — the
backend has its own keys and commands work without BYOK.

## The agent protocol — when keys are missing or rejected

Failed commands carry a `providerKeysHint` in the JSON error envelope:

```json
"providerKeysHint": {
  "code": "provider_keys_required",      // or provider_keys_invalid
  "provider": "epo",
  "requiresHumanIntervention": true,
  "nonInteractive": { "command": "flowleap keys set epo --key … --secret …",
                       "env": ["FLOWLEAP_EPO_KEY", "FLOWLEAP_EPO_SECRET"] },
  "signup": "https://developers.epo.org (free, 'My apps' → create app)"
}
```

**Getting keys requires a browser signup — an agent cannot complete this
alone. Do not retry, do not invent keys.** Tell the user:

> This command needs EPO OPS credentials. Please run `flowleap setup` in a
> terminal (guided, ~2 minutes; free keys from https://developers.epo.org),
> then I'll continue.

If the user hands you keys directly, apply them non-interactively — they are
validated live before saving, and rejected keys are NOT saved:

```bash
flowleap --json keys set epo --key <consumer-key> --secret <consumer-secret>
flowleap --json keys set uspto --key <api-key>
flowleap --json keys test
```

Or per-session via env: `FLOWLEAP_EPO_KEY`, `FLOWLEAP_EPO_SECRET`,
`FLOWLEAP_USPTO_KEY`.

## Human commands (mention, never run yourself)

`flowleap setup` — full onboarding wizard (backend check → auth check →
per-provider prompts with hidden input, live validation, skippable steps with
explicit warnings). Refuses to run without a TTY. `flowleap keys rm epo|uspto`
removes stored keys.
