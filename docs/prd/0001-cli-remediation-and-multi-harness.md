# PRD 0001: CLI Remediation + Multi-Harness Expansion — MCP Server, Hardened Client, Honest Funnel

## Problem Statement

Developers and AI agents outside the FlowLeap IDE are the CLI's entire market, yet today:

- An agent calling the CLI against a hung backend **waits forever** — there is no HTTP timeout anywhere on the main request path.
- Every failure exits with the same code, so an agent cannot distinguish "log in again" from "subscribe" from "back off and retry" without parsing JSON; and the subscription paywall (402) surfaces as a bare "API error (402)" with no guidance, killing trial conversion at the exact moment a prospect hits the wall.
- Pointing the CLI at a mistyped or malicious base URL silently sends the user's session token, personal token, **and** their EPO/USPTO provider keys to that host.
- The curl|sh installer downloads the binary without any checksum verification, while the npm path carefully verifies sha256 — the least-defended install path is also the most casual one. Nothing runs the test suite in CI; only releases have a workflow.
- Despite the multi-harness mission, the CLI is Claude-only in practice: skills install only to Claude-convention paths, no other harness can auto-load the SKILL.md format, and there is **no MCP server** — the one integration that would make Codex, Cursor, opencode, Cline, and Gemini CLI first-class overnight.
- The flagship analytics endpoint shipped 2026-07-07 (`/v1/patent-analytics`) is reachable only through the raw API escape hatch — no command, not even in the tools facade. OCR and claim analysis are in the same state.
- The README — the actual adoption funnel — never mentions the agent-first `tools` facade, the `skills install` command, five of sixteen commands, or the fact that using the CLI requires an account with an active **Basic plan** (founder-confirmed: the docs claiming free data are stale; the guards are correct). Several skills document an auth flow (PKCE) and token format (`sk-...`) that don't exist.

## Solution

Harden the one HTTP seam every command shares, ship a stdio **MCP server** that bridges the backend's self-describing tools facade to every MCP-capable harness, close the coverage gaps with ergonomic verbs (analytics first), make the skills installer genuinely multi-harness with versioning, and rewrite the README around the truth: what the tool does for agents, how skills/MCP get installed per harness, and what it costs (auth + Basic plan, pricing at flowleap.co/pricing). Guard it all with CI and a skill↔CLI validation test so drift fails the build instead of misleading agents.

## User Stories

1. As an AI agent calling the CLI, I want every request to time out with a clear error, so that a hung backend never blocks my run forever.
2. As an AI agent, I want distinct exit codes for auth-required, subscription-required, not-found, rate-limited, and network-unreachable, so that I can branch on `$?` without parsing JSON.
3. As a trial prospect hitting the paywall, I want the 402 response to come with a structured hint (what plan I need, where to subscribe), so that I convert instead of concluding the tool is broken.
4. As an agent hitting a rate limit, I want a structured hint carrying the retry-after seconds, so that I back off precisely instead of hammering or giving up.
5. As a security-conscious user, I want a loud warning (and confirmation in interactive mode) before my credentials are sent to a non-FlowLeap host, so that a typo'd base URL can't exfiltrate my tokens and provider keys.
6. As a user installing via curl|sh, I want the installer to verify the binary's sha256 against the published checksums, so that the casual install path is as safe as npm.
7. As a maintainer, I want CI running tests, clippy, and fmt on every PR — and asserting the Cargo version matches the release tag — so that regressions and version drift are caught before merge.
8. As a Cursor/Codex/opencode/Gemini user, I want `flowleap mcp` to run a stdio MCP server exposing every backend tool, so that my harness gets FlowLeap natively with zero skill files.
9. As an MCP client, I want the server's tool list and input schemas to come live from the backend's tools facade, so that new backend tools appear without a CLI release.
10. As an IP analyst, I want a `flowleap analytics` command with structured flags (keywords, phrases, assignee, CPC/IPC, country, date range), so that full-corpus landscape data is one command away instead of a hand-built raw request.
11. As an agent, I want the analytics, claim-analysis, and OCR capabilities registered in the backend tools facade, so that both `tools run` and the MCP server can reach them.
12. As a researcher, I want ergonomic verbs for compare, figures, summary, prosecution timeline, and patent-number conversion, so that the highest-value facade tools don't require knowing the facade exists.
13. As a paralegal with a scanned patent PDF, I want a `flowleap ocr` command accepting a file or URL, so that I don't have to hand-roll base64 into a raw API request.
14. As an attorney, I want `flowleap analyze-claim` to accept claim text (or stdin), so that claim structure analysis works from any shell or script.
15. As a Codex/Cursor/Gemini user, I want skill installation to emit the format my harness actually auto-loads (AGENTS.md, rules files, GEMINI.md), so that "multi-harness support" is real rather than a directory dump.
16. As a returning user, I want installed skills to carry a version stamp and a `skills update` path (or auto-refresh on binary update), so that a CLI upgrade doesn't leave stale instructions in my harness.
17. As an agent routed by skill descriptions, I want every persona and recipe skill to carry a rich "trigger when…" description, so that routing works as well for workflows as it does for service commands.
18. As a patent professional, I want recipes for the prosecution/litigation cluster (office-action response, invalidity, infringement charting, claim drafting, invention disclosure, audit trail), so that the CLI covers the workflows the IDE already does.
19. As a maintainer, I want a test that parses every command example in every skill and validates it against the real CLI surface, so that skill drift fails CI instead of misleading agents.
20. As a prospect reading the README, I want the agent-first tools facade, the skills installer, and the full command table documented, so that the product's actual value proposition is visible before install.
21. As a prospect, I want the README to state plainly that the CLI requires an account with an active Basic plan (pricing at flowleap.co/pricing, trial available), so that the 402 I might hit is expected rather than a betrayal.
22. As a CI/headless user, I want a documented non-interactive auth path (create-token, env vars), so that pipelines authenticate without a browser.
23. As an evaluating agent, I want an offline capabilities listing that doesn't require auth, so that I can discover what the CLI offers before sign-in.
24. As a new user, I want the auth docs and skills to describe the real device flow and real token format (`fl_pat_...`), so that examples work verbatim.
25. As a cargo/curl installer, I want the update notice to give the command for *my* install channel, so that following it doesn't create a second conflicting install.
26. As anyone consuming the project, I want a LICENSE file matching the declared MIT license, so that legal status is unambiguous.
27. As an Alpine/container user, I want a static musl Linux build in releases, so that the CLI runs in minimal CI images.
28. As an agent reading errors, I want the CLI to send an identifying User-Agent, so that the backend can version-gate and debug client issues.
29. As an agent on a flaky network, I want bounded retry with backoff on transient failures (5xx/network), so that one blip doesn't fail a workflow step.

## Implementation Decisions

- **All transport hardening lands at the single shared HTTP/Context seam** every command uses: default request timeout (~30s) and connect timeout (~5s), overridable by env; a versioned User-Agent; bounded jittered retry (2 attempts) on network errors and 5xx for read-only calls; 429 respected via Retry-After. The device-flow poller and update checker reuse the same configured client rather than constructing their own.
- **Exit-code contract** (documented in the README and agents guide): 0 success; 1 generic; dedicated codes for auth-required (401), subscription-required (402), not-found (404), rate-limited (429), network/timeout. The JSON error envelope is unchanged (additive only) so existing consumers keep working.
- **Structured hints extend the existing provider-keys hint pattern**: a subscription hint on 402 (plan requirement, upgrade URL from the response or flowleap.co/pricing, `requiresHumanIntervention: true`) and a rate-limit hint on 429 (carrying retry-after seconds). Human mode renders the same info as the existing stderr info box.
- **Base-URL guard**: when the effective base URL's host is not `*.flowleap.co` (or localhost/127.0.0.1 for dev), print a prominent stderr warning naming the host and which credentials will be sent; in interactive TTY mode require confirmation; `--yes`/non-TTY proceeds with the warning only. No allowlist config in this PRD.
- **`flowleap mcp` is a thin stdio bridge over the existing tools-facade module**: MCP `tools/list` ← facade list (names, descriptions, input schemas passed through verbatim — **the `/v1/tools` vocabulary is canonical**); MCP `tools/call` ← facade run; facade errors map to MCP tool errors with the structured hints embedded in the error content. No per-tool code, no name mapping. Auth reuses stored credentials; unauthenticated startup returns a clear MCP error instructing `flowleap auth login`. Reconciling the separately-hosted claude.ai MCP's divergent names is a backend/website follow-up, out of scope here.
- **New verbs wrap existing backend contracts**: `analytics` (structured flags mirroring the endpoint contract: keywords/phrases/assignee/cpc/ipc/country/date range; rejecting none-provided locally), `ocr` (file path or URL), `analyze-claim` (text arg or stdin), plus facade-backed `compare`, `figures`, `summary`, `timeline`, `convert-number`. All follow the existing command pattern: `--json` envelope, human formatter with real columns.
- **Backend (cross-repo)**: register `patent_analytics`, `analyze_claim`, and `ocr` in the `/v1/tools` registry with JSON schemas mirroring their route contracts, so the facade, the MCP server, and `tools list` all surface them without CLI changes.
- **Skills installer becomes target-aware**: named harness targets (claude user/project, codex, cursor, gemini, generic dir). Non-Claude targets emit what that harness auto-loads — a generated agents-rules document (AGENTS.md / GEMINI.md / rules file) rendered from the same embedded skill content, condensed to command reference + workflow triggers. Installed output carries the CLI version stamp; `skills update` refreshes previously-installed targets (recorded in config); the update notice mentions stale skills when the binary version advances past the stamp.
- **Skill content normalization**: single entry-point skill (the umbrella absorbs the shared/start-here role); all personas/recipes upgraded to rich trigger-style descriptions with the dependency metadata retained; the misleading PKCE/`sk-...` content corrected to device flow and `fl_pat_...`; the under-documented academic flags completed. Six new prosecution/litigation recipes authored against existing facade tools (office-action response, invalidity, infringement charting, claim drafting, invention disclosure, audit trail).
- **README/funnel rewrite**: full command table (all sixteen), the tools facade and MCP server front and center, per-harness quickstarts, the auth + Basic plan requirement with flowleap.co/pricing link and trial mention, CI/headless auth guide, JSON envelope + exit-code reference, cookbook section surfacing the recipes. The stale aspirational PKCE plan doc is deleted or archived; the dead refresh-token field removed; LICENSE (MIT) added.
- **Distribution**: install.sh verifies sha256 against the release checksums before installing; release workflow gains a version-consistency assertion (tag == Cargo version == npm version) and a linux-musl static target; the update notice detects install channel (npm wrapper marker vs standalone) and prints the matching upgrade command. Homebrew tap and macOS notarization are out of scope.
- **CI**: a workflow running fmt, clippy (deny warnings), tests, and the skill-validation test on every PR/push.

## Testing Decisions

- Test at the highest seam: HTTP behavior (timeout, retry, 429, envelope shape, exit codes, hints) via a mock HTTP server (wiremock) asserting on the CLI's JSON envelope and process exit codes — external behavior, not internals. Prior art: the existing dry-run CLI tests and config/credentials unit tests.
- The MCP server gets a protocol-level test: spawn the server on stdio with a mocked facade backend, assert `tools/list` mirrors the facade schemas verbatim and `tools/call` round-trips a run, including the error path with hints.
- **Skill↔CLI validation test**: parse every fenced `flowleap …` example in every embedded skill and validate it against the clap command tree; runs in the normal test suite so drift fails CI. Also asserts every skill has a non-trivial trigger-style description.
- Installer targets get golden-file tests: each harness target's generated output is snapshot-asserted from the embedded content.
- Exit-code contract asserted per status class in the wiremock suite.
- install.sh checksum logic tested in CI by installing a real published artifact against its checksums file (or a fixture server if the live hit is flaky).

## Out of Scope

- Reconciling the hosted claude.ai FlowLeap MCP's tool names with the facade vocabulary (backend/website follow-up).
- Homebrew tap, macOS code signing/notarization, Windows installer polish.
- Token refresh / a new OAuth flow (device flow + 401→API-key fallback stays; the dead refresh-token field is simply removed).
- Multi-profile support (`--profile`).
- Offline response caching, streaming/size-capped response handling.
- Backend pricing/trial mechanics themselves (Basic plan gating is correct as-is; only docs change).
- The IDE-side skill libraries in flowleap-agent-v2.

## Further Notes

- Founder decision recorded 2026-07-08: the CLI requires auth + an active **Basic plan**; "data free forever" docs claims are stale and must be corrected in docs, not by loosening guards. README links flowleap.co/pricing.
- MCP vocabulary decision: the backend `/v1/tools` names are canonical (`search_patents`, `get_claims`, …). The CLI must not invent a third naming surface.
- Cross-repo: the three tools-registry additions live in `flowleap-backend`; slice them separately so agents stay in one repo per issue.
- Review provenance: four-agent survey 2026-07-08; findings index in project memory note `flowleap-cli-review-2026-07-08`.
