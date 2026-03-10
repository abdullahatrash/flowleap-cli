# OAuth PKCE Flow Fix — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the CLI OAuth login flow so the authorization code + PKCE exchange works end-to-end across CLI, website, and backend.

**Architecture:** The CLI currently sends the browser directly to the backend's `/oauth/authorize`, which only accepts `response_type=token`. The website already has a `/oauth/authorize` page built for the PKCE code flow but the backend is missing the two endpoints it calls: `POST /oauth/authorize` (generate auth code) and `POST /oauth/token` (exchange code for token). Fix: (1) add missing backend endpoints, (2) update the backend GET `/oauth/authorize` to pass PKCE params, (3) point CLI at the website for the authorize step.

**Tech Stack:** Rust (CLI), TypeScript/Express (backend), Astro (website), Clerk (auth provider)

**Branches:**
- `flowleap-backend`: `fix/oauth-pkce-flow`
- `flowleap-website`: `fix/oauth-pkce-flow`
- `flowleap-cli`: `claude/implement-flowleap-cli-3MWfx` (current)

**Repos:**
- CLI: `/Users/neoak/projects/flowleap-cli`
- Backend: `/Users/neoak/projects/flowleap-backend`
- Website: `/Users/neoak/projects/flowleap-website`

---

## Task 1: Backend — Add authorization code storage and generation

**Files:**
- Modify: `/Users/neoak/projects/flowleap-backend/src/routes/oauth.ts`

**Context:** The website's `/oauth/authorize` page (already built) calls `POST /oauth/authorize` on the backend with a Clerk bearer token + PKCE params to generate a short-lived authorization code. This endpoint does not exist yet.

**Step 1: Add authorization code in-memory store at the top of oauth.ts**

After the `REGISTERED_CLIENTS` map and helper functions (after line 75), add:

```typescript
import crypto from 'crypto';

// In-memory authorization code store
// Maps code -> { clientId, redirectUri, codeChallenge, codeChallengeMethod, clerkToken, expiresAt }
interface AuthorizationCode {
	clientId: string;
	redirectUri: string;
	codeChallenge: string;
	codeChallengeMethod: string;
	clerkToken: string;
	expiresAt: number;
}

const authorizationCodes = new Map<string, AuthorizationCode>();

// Cleanup expired codes every 60 seconds
setInterval(() => {
	const now = Date.now();
	for (const [code, data] of authorizationCodes) {
		if (data.expiresAt < now) {
			authorizationCodes.delete(code);
		}
	}
}, 60_000);
```

**Step 2: Add POST `/oauth/authorize` endpoint**

Add after the existing GET `/oauth/authorize` handler (after line 184):

```typescript
// POST /oauth/authorize — Generate authorization code (called by website after Clerk auth)
oauthRouter.post('/authorize', async (req: Request, res: Response) => {
	try {
		const auth = getAuth(req);
		if (!auth?.userId) {
			return res.status(401).json({
				error: 'unauthorized',
				error_description: 'Authentication required',
			});
		}

		const { client_id, redirect_uri, state, response_type, code_challenge, code_challenge_method } = req.body;

		// Validate required fields
		if (!client_id || !redirect_uri || !code_challenge || !code_challenge_method) {
			return res.status(400).json({
				error: 'invalid_request',
				error_description: 'Missing required parameters',
			});
		}

		if (response_type !== 'code') {
			return res.status(400).json({
				error: 'unsupported_response_type',
				error_description: 'Only "code" response type is supported for this endpoint',
			});
		}

		if (code_challenge_method !== 'S256') {
			return res.status(400).json({
				error: 'invalid_request',
				error_description: 'Only S256 code_challenge_method is supported',
			});
		}

		// Validate client
		if (!REGISTERED_CLIENTS.has(client_id)) {
			return res.status(400).json({
				error: 'invalid_client',
				error_description: 'Unknown client_id',
			});
		}

		if (!isValidRedirectUri(client_id, redirect_uri)) {
			return res.status(400).json({
				error: 'invalid_request',
				error_description: 'Invalid redirect_uri for this client',
			});
		}

		// Get the Clerk session token from Authorization header
		const authHeader = req.headers.authorization;
		const clerkToken = authHeader?.replace('Bearer ', '') || '';

		// Generate authorization code
		const code = crypto.randomBytes(32).toString('hex');

		authorizationCodes.set(code, {
			clientId: client_id,
			redirectUri: redirect_uri,
			codeChallenge: code_challenge,
			codeChallengeMethod: code_challenge_method,
			clerkToken,
			expiresAt: Date.now() + 5 * 60 * 1000, // 5 minutes
		});

		res.json({ code });
	} catch (error) {
		console.error('Authorization code generation error:', error);
		res.status(500).json({
			error: 'server_error',
			error_description: 'Internal server error',
		});
	}
});
```

**Step 3: Run backend tests**

Run: `cd /Users/neoak/projects/flowleap-backend && npm test`
Expected: Existing tests still pass.

**Step 4: Commit**

```bash
cd /Users/neoak/projects/flowleap-backend
git add src/routes/oauth.ts
git commit -m "feat: add POST /oauth/authorize for PKCE authorization code generation"
```

---

## Task 2: Backend — Add token exchange endpoint

**Files:**
- Modify: `/Users/neoak/projects/flowleap-backend/src/routes/oauth.ts`

**Context:** After the CLI receives an authorization code, it exchanges it for a token by calling `POST /oauth/token` with the code + PKCE code_verifier. The backend must verify the PKCE challenge and return the stored Clerk token.

**Step 1: Add POST `/oauth/token` endpoint**

Add after the POST `/oauth/authorize` handler:

```typescript
// POST /oauth/token — Exchange authorization code + PKCE verifier for access token
oauthRouter.post('/token', (req: Request, res: Response) => {
	try {
		const { grant_type, code, redirect_uri, client_id, code_verifier } = req.body;

		if (!grant_type || !code || !redirect_uri || !client_id || !code_verifier) {
			return res.status(400).json({
				error: 'invalid_request',
				error_description: 'Missing required parameters: grant_type, code, redirect_uri, client_id, code_verifier',
			});
		}

		if (grant_type !== 'authorization_code') {
			return res.status(400).json({
				error: 'unsupported_grant_type',
				error_description: 'Only "authorization_code" grant type is supported',
			});
		}

		// Look up and consume the authorization code (single-use)
		const authCode = authorizationCodes.get(code);
		if (!authCode) {
			return res.status(400).json({
				error: 'invalid_grant',
				error_description: 'Invalid or expired authorization code',
			});
		}
		authorizationCodes.delete(code);

		// Check expiry
		if (authCode.expiresAt < Date.now()) {
			return res.status(400).json({
				error: 'invalid_grant',
				error_description: 'Authorization code has expired',
			});
		}

		// Validate client_id and redirect_uri match original request
		if (authCode.clientId !== client_id || authCode.redirectUri !== redirect_uri) {
			return res.status(400).json({
				error: 'invalid_grant',
				error_description: 'client_id or redirect_uri mismatch',
			});
		}

		// Verify PKCE: SHA256(code_verifier) must match stored code_challenge
		const hash = crypto.createHash('sha256').update(code_verifier).digest();
		const expectedChallenge = hash.toString('base64url');

		if (expectedChallenge !== authCode.codeChallenge) {
			return res.status(400).json({
				error: 'invalid_grant',
				error_description: 'PKCE verification failed',
			});
		}

		// Return the stored Clerk token as the access token
		res.json({
			access_token: authCode.clerkToken,
			token_type: 'Bearer',
		});
	} catch (error) {
		console.error('Token exchange error:', error);
		res.status(500).json({
			error: 'server_error',
			error_description: 'Internal server error',
		});
	}
});
```

**Step 2: Run backend tests**

Run: `cd /Users/neoak/projects/flowleap-backend && npm test`
Expected: Existing tests still pass.

**Step 3: Commit**

```bash
cd /Users/neoak/projects/flowleap-backend
git add src/routes/oauth.ts
git commit -m "feat: add POST /oauth/token for PKCE code exchange"
```

---

## Task 3: Backend — Update GET `/oauth/authorize` to support `response_type=code`

**Files:**
- Modify: `/Users/neoak/projects/flowleap-backend/src/routes/oauth.ts`

**Context:** The GET `/oauth/authorize` currently rejects `response_type=code`. It also redirects to `/en/auth/flowleap` (the VS Code extension page) without passing PKCE params. Update it to: (1) accept both `token` and `code`, (2) redirect to the correct website page based on flow, (3) pass PKCE params when present.

**Step 1: Update the response_type validation (line 147-152)**

Replace:
```typescript
if (response_type && response_type !== 'token') {
    return res.status(400).json({
        error: 'unsupported_response_type',
        error_description: 'Only "token" response type is supported',
    });
}
```

With:
```typescript
if (response_type && response_type !== 'token' && response_type !== 'code') {
    return res.status(400).json({
        error: 'unsupported_response_type',
        error_description: 'Supported response types: "token", "code"',
    });
}
```

**Step 2: Update the redirect to pass PKCE params and route correctly (lines 170-176)**

Replace:
```typescript
// Redirect to website for Clerk sign-in (use locale-prefixed path)
const frontendUrl = process.env.FRONTEND_URL || 'http://localhost:4321';
const signInUrl = new URL(`${frontendUrl}/en/auth/flowleap`);
signInUrl.searchParams.set('redirect_uri', redirect_uri as string);
signInUrl.searchParams.set('state', state as string);

res.redirect(signInUrl.toString());
```

With:
```typescript
const frontendUrl = process.env.FRONTEND_URL || 'http://localhost:4321';

if (response_type === 'code') {
    // PKCE authorization code flow (CLI) — redirect to website's OAuth authorize page
    const { code_challenge, code_challenge_method } = req.query;
    if (!code_challenge || !code_challenge_method) {
        return res.status(400).json({
            error: 'invalid_request',
            error_description: 'PKCE parameters required for code flow: code_challenge, code_challenge_method',
        });
    }
    const oauthUrl = new URL(`${frontendUrl}/oauth/authorize`);
    oauthUrl.searchParams.set('client_id', client_id as string);
    oauthUrl.searchParams.set('redirect_uri', redirect_uri as string);
    oauthUrl.searchParams.set('state', state as string);
    oauthUrl.searchParams.set('response_type', 'code');
    oauthUrl.searchParams.set('code_challenge', code_challenge as string);
    oauthUrl.searchParams.set('code_challenge_method', code_challenge_method as string);
    res.redirect(oauthUrl.toString());
} else {
    // Implicit token flow (VS Code extension) — redirect to auth page
    const signInUrl = new URL(`${frontendUrl}/en/auth/flowleap`);
    signInUrl.searchParams.set('redirect_uri', redirect_uri as string);
    signInUrl.searchParams.set('state', state as string);
    res.redirect(signInUrl.toString());
}
```

**Step 3: Update the OpenAPI docs for the endpoint (lines 107-111)**

Replace:
```
 *           enum: [token]
 *         description: OAuth response type (only "token" is supported)
```

With:
```
 *           enum: [token, code]
 *         description: OAuth response type ("token" for implicit, "code" for PKCE)
```

And add PKCE parameter docs after the state parameter:
```
 *       - in: query
 *         name: code_challenge
 *         schema:
 *           type: string
 *         description: PKCE code challenge (required when response_type=code)
 *       - in: query
 *         name: code_challenge_method
 *         schema:
 *           type: string
 *           enum: [S256]
 *         description: PKCE challenge method (required when response_type=code)
```

**Step 4: Run backend tests**

Run: `cd /Users/neoak/projects/flowleap-backend && npm test`
Expected: All tests pass.

**Step 5: Commit**

```bash
cd /Users/neoak/projects/flowleap-backend
git add src/routes/oauth.ts
git commit -m "feat: update GET /oauth/authorize to support response_type=code with PKCE"
```

---

## Task 4: CLI — Point authorize URL to website instead of backend

**Files:**
- Modify: `/Users/neoak/projects/flowleap-cli/src/config.rs`
- Modify: `/Users/neoak/projects/flowleap-cli/src/commands/auth.rs`
- Modify: `/Users/neoak/projects/flowleap-cli/src/main.rs` (if CLI flag/env var needed)

**Context:** The CLI currently sends the browser to `{base_url}/oauth/authorize` where `base_url` is the backend (`api.flowleap.co`). The authorize step must go to the website (`flowleap.co`), while the token exchange stays at the backend. We need a `website_url` config.

**Step 1: Add `website_url` to Config in `config.rs`**

Add field to the Config struct:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_website_url")]
    pub website_url: String,
    pub default_model: Option<String>,
    pub output_format: Option<String>,
}
```

Add default function:
```rust
fn default_website_url() -> String {
    "https://flowleap.co".to_string()
}
```

Update the Default impl:
```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            website_url: default_website_url(),
            default_model: None,
            output_format: None,
        }
    }
}
```

**Step 2: Update auth.rs to use website_url for authorize, base_url for token exchange**

In `auth.rs`, change lines 95-103 from:
```rust
let base_url = ctx.config.base_url.trim_end_matches('/');
let auth_url = format!(
    "{}/oauth/authorize?client_id={}&redirect_uri={}&state={}&response_type=code&code_challenge={}&code_challenge_method=S256",
    base_url,
    CLIENT_ID,
    urlencoding(&redirect_uri),
    state,
    challenge
);
```

To:
```rust
let base_url = ctx.config.base_url.trim_end_matches('/');
let website_url = ctx.config.website_url.trim_end_matches('/');
let auth_url = format!(
    "{}/oauth/authorize?client_id={}&redirect_uri={}&state={}&response_type=code&code_challenge={}&code_challenge_method=S256",
    website_url,
    CLIENT_ID,
    urlencoding(&redirect_uri),
    state,
    challenge
);
```

The token exchange on line 122 stays pointing to `base_url`:
```rust
let token_url = format!("{}/oauth/token", base_url);
```
This is correct — no change needed.

**Step 3: Add `--website-url` flag and `FLOWLEAP_WEBSITE_URL` env var support in main.rs**

Check `main.rs` for how `base_url` is overridden and follow the same pattern for `website_url`. Add:
- CLI flag: `--website-url <URL>`
- Env var: `FLOWLEAP_WEBSITE_URL`

**Step 4: Build and verify**

Run: `cd /Users/neoak/projects/flowleap-cli && cargo build`
Expected: Compiles with zero errors.

**Step 5: Run tests**

Run: `cd /Users/neoak/projects/flowleap-cli && cargo test`
Expected: All tests pass.

**Step 6: Run lints**

Run: `cd /Users/neoak/projects/flowleap-cli && cargo clippy && cargo fmt --check`
Expected: Zero warnings, formatting clean.

**Step 7: Commit**

```bash
cd /Users/neoak/projects/flowleap-cli
git add src/config.rs src/commands/auth.rs src/main.rs
git commit -m "fix: point OAuth authorize to website URL for PKCE flow"
```

---

## Task 5: End-to-end verification

**Step 1: Start backend locally**

```bash
cd /Users/neoak/projects/flowleap-backend && npm run dev
```

**Step 2: Start website locally**

```bash
cd /Users/neoak/projects/flowleap-website && npm run dev
```

**Step 3: Test CLI login with local services**

```bash
cd /Users/neoak/projects/flowleap-cli
cargo run -- --base-url http://localhost:8000 --website-url http://localhost:4321 auth login
```

Expected flow:
1. Browser opens to `http://localhost:4321/oauth/authorize?client_id=flowleap-cli&...`
2. Website shows Clerk sign-in (or redirects to sign-in)
3. After auth, website calls backend `POST /oauth/authorize` → gets auth code
4. Website redirects to `http://127.0.0.1:<port>/callback?code=<code>&state=<state>`
5. CLI receives code, calls backend `POST /oauth/token` with code + code_verifier
6. Backend validates PKCE, returns Clerk token
7. CLI stores token in `~/.config/flowleap/credentials.toml`
8. CLI prints "Successfully authenticated!"

**Step 4: Verify auth status**

```bash
cargo run -- --base-url http://localhost:8000 auth status
```

Expected: Shows "Authenticated (token)" with user email.

---

## Task 6: Cleanup — Remove implicit flow dead code from CLI (optional)

**Files:**
- Modify: `/Users/neoak/projects/flowleap-cli/src/commands/auth.rs`

**Context:** The CLI has `CallbackResult::Token` variant and implicit flow handling (lines 155-160, 240-241) that is no longer used since the CLI exclusively uses the authorization code flow. This can be removed for cleanliness.

**Step 1: Remove `CallbackResult::Token` variant and its handling**

Remove from `login()` (lines 155-160):
```rust
CallbackResult::Token(token_value) => {
    // Implicit flow — token returned directly (Clerk session token)
    let mut creds = Credentials::load()?;
    creds.token = Some(token_value);
    creds.save()?;
}
```

Remove from `wait_for_callback()` (lines 240-241):
```rust
if let Some((_, token)) = params.iter().find(|(k, _)| k == "access_token") {
    return Ok(CallbackResult::Token(token.clone()));
}
```

Remove the `Token` variant from `CallbackResult` enum and simplify to just return the code `String` directly (no enum needed anymore).

**Step 2: Build, test, lint**

Run: `cd /Users/neoak/projects/flowleap-cli && cargo build && cargo test && cargo clippy && cargo fmt --check`

**Step 3: Commit**

```bash
cd /Users/neoak/projects/flowleap-cli
git add src/commands/auth.rs
git commit -m "refactor: remove unused implicit flow code from CLI auth"
```
