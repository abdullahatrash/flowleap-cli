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
flowleap auth logout
```

Clears all stored credentials.

## Environment Variable Override

Set `FLOWLEAP_API_KEY` or `FLOWLEAP_TOKEN` to bypass stored credentials entirely. These take effect without running `auth login`.
