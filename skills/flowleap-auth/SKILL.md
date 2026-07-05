---
name: flowleap-auth
version: 1.0.0
description: "FlowLeap Auth: OAuth 2.0 + PKCE login, API key auth, and status."
metadata:
  category: "patent-ai"
  requires:
    bins: ["flowleap"]
  cliHelp: "flowleap auth --help"
---

# FlowLeap Auth

Prerequisite: Read `flowleap-shared` for global flags and configuration.

## Commands

### Login via OAuth (opens browser)

```bash
flowleap auth login
```

Starts OAuth 2.0 + PKCE flow: opens browser, runs local callback server, exchanges code for JWT, stores in `~/.config/flowleap/credentials.toml`.

### Login with API Key

```bash
flowleap auth login --api-key sk-your-key-here
```

### Login with Token

```bash
flowleap auth login --token eyJhbGci...
```

### Check Status

```bash
flowleap auth status
```

Shows: base URL, authentication method, default model, and user profile (if authenticated).

### Logout

```bash
flowleap auth logout                # clear everything, including provider keys
flowleap auth logout --session-only # clear only the OAuth session token
```

Plain `logout` clears all stored credentials, including EPO/USPTO provider
keys. Use `--session-only` to drop just the browser-session token — useful
when an expired session token is shadowing a still-valid `fl_pat_` API key
(the CLI prefers the session token when both are stored).

## Environment Variable Override

Set `FLOWLEAP_API_KEY` or `FLOWLEAP_TOKEN` to bypass stored credentials entirely. These take effect without running `auth login`.
