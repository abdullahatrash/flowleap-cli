---
name: flowleap-auth
description: Authenticate the FlowLeap CLI — OAuth 2.0 device flow login (user code + verification URL), long-lived fl_pat_ personal API tokens for headless agents, status checks, and targeted logout. Trigger when a FlowLeap command fails with 401/unauthenticated, when setting up credentials for an agent or CI, or when the user asks to log in to FlowLeap or mint, list, or revoke API tokens.
---

# FlowLeap Auth

Global flags and configuration: see `flowleap-shared`.

## Commands

### Login via OAuth device flow

```bash
flowleap auth login
```

Starts the OAuth 2.0 device flow: the CLI requests a device code from the
backend, prints a user code plus verification URL (and opens the browser),
then polls until the login is approved. The resulting session JWT is stored
in `~/.config/flowleap/credentials.toml` (mode 0600). No local callback
server is involved, so it also works when the browser runs on another
machine — open the printed URL and enter the user code there.

### Login with a personal API token

```bash
flowleap auth login --api-key fl_pat_your_token_here
```

### Login with a session token

```bash
flowleap auth login --token eyJhbGci...
```

### Personal API tokens (headless/agent use)

```bash
flowleap auth create-token --name my-agent --store   # mint + store (shown once)
flowleap auth tokens                                 # list tokens
flowleap auth revoke-token <id>                      # revoke by id
```

Personal tokens use the `fl_pat_…` format and are long-lived. Minting
requires an OAuth session — API tokens cannot mint further tokens
(backend-enforced).

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

Since v0.2.3 the CLI self-heals this case: on a 401 with a stored session
token, it retries once with the stored API key and prints a stderr warning
suggesting `logout --session-only`. The fallback is skipped when the token
was passed explicitly via `--token` or `FLOWLEAP_TOKEN`.

## Environment Variable Override

Set `FLOWLEAP_API_KEY` (an `fl_pat_…` token) or `FLOWLEAP_TOKEN` to bypass
stored credentials entirely. These take effect without running `auth login`.
