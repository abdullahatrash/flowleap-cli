<!-- BEGIN FLOWLEAP AGENT RULES (flowleap-cli v0.0.0-test) -->
## FlowLeap CLI — Agent Rules

Rendered by flowleap-cli v0.0.0-test from its bundled agent skills. Refresh after a CLI upgrade with `flowleap skills update`.

FlowLeap is a CLI for the FlowLeap Patent AI backend: patent/USPTO/OPS/academic/NPL/legal/citation search plus an agent-first tools facade.

- Add `--json` to every command for stable machine-readable output.
- Run `flowleap --json doctor` first to verify config, auth, and backend reachability.
- Authenticate with `flowleap auth login`, or set `FLOWLEAP_API_KEY`/`FLOWLEAP_TOKEN` for headless use.
- If an error carries a `providerKeysHint` (`provider_keys_required` / `provider_keys_invalid`), stop — this needs a human. Ask the user to run `flowleap setup`; do not retry or invent keys.
- Discover every backend tool with `flowleap tools list`; run one with `flowleap tools run <name>`.

### Command Reference

#### flowleap — Use the installed FlowLeap CLI to inspect FlowLeap Patent AI backend health, authenticate safely, run patent/USPTO/OPS/academic/NPL/legal/citation reads, and use the raw API escape hatch.

```bash
flowleap --json doctor
flowleap --json doctor --base-url http://localhost:8000
flowleap --json health --base-url http://localhost:8000
flowleap --json health cache --base-url http://localhost:8000
flowleap --json doctor --base-url https://api.flowleap.co
flowleap auth login --api-key ...
flowleap --json api profile
flowleap auth create-token --name my-agent --store
```

#### flowleap-academic — FlowLeap Academic: Search academic literature.

```bash
flowleap academic search <query> [flags]
```

#### flowleap-auth — FlowLeap Auth: OAuth 2.0 + PKCE login, API key auth, and status.

```bash
flowleap auth login
flowleap auth status
flowleap auth logout
```

#### flowleap-citation — USPTO enriched citation data from office actions — citations by application, forward citations of a document, citation statistics and X-category novelty-destroying references.

```bash
flowleap --json citation search 16123456 --size 20
flowleap --json citation forward US10123456 --size 20
flowleap --json citation stats 16123456
flowleap --json citation novelty 16123456
```

#### flowleap-keys — Manage BYOK patent-provider credentials (EPO OPS consumer key/secret, USPTO ODP API key) for the FlowLeap CLI — check status, validate live, and hand off to a human for the interactive setup wizard.

```bash
flowleap --json keys list
flowleap --json keys test
flowleap --json doctor
flowleap --json keys set epo --key <consumer-key> --secret <consumer-secret>
```

#### flowleap-legal — Hybrid semantic/keyword search over patent-law reference documents (EPC, EPO Guidelines, MPEP, EU and WIPO materials) with jurisdiction filters.

```bash
flowleap --json legal search "inventive step problem solution approach" --jurisdiction epo --limit 5
flowleap --json legal jurisdictions
flowleap --json legal stats
flowleap --json legal docs
```

#### flowleap-npl — Search non-patent literature (scholarly works via OpenAlex) with year, open-access and publication-type filters.

```bash
flowleap --json npl "perovskite solar cell stability" --limit 5
flowleap --json npl "CRISPR delivery" --from-year 2020 --to-year 2024 --open-access
flowleap --json npl "transformer attention" --type preprint
```

#### flowleap-ops — FlowLeap OPS: Direct EPO Open Patent Services API access.

```bash
flowleap ops search --cql <query> [flags]
flowleap ops biblio <doc>
flowleap ops claims <doc> [--lang en]
flowleap ops description <doc> [--lang en]
flowleap ops family <doc>
flowleap ops legal <doc>
flowleap ops abstract <doc>
```

#### flowleap-patent — FlowLeap Patent: Search patents and build CQL queries.

```bash
flowleap patent search --query <query> [flags]
flowleap uspto search --query "lithium battery" --limit 20
flowleap patent build-query <description> [flags]
```

#### flowleap-shared — FlowLeap CLI: Shared authentication, configuration, and global flags.

```bash
flowleap auth login
flowleap auth status
flowleap auth logout
flowleap config set base-url https://api.flowleap.co
flowleap config list
flowleap config reset
```

#### flowleap-tools — Discover and run FlowLeap backend tools through the agent-first /v1/tools facade — one stable contract (search_patents, get_bibliography, get_claims, get_patent_summary, compare_patents, reference_search, …) instead of provider-specific endpoints.

```bash
flowleap --json tools list
flowleap --json tools describe get_bibliography
flowleap --json tools openapi
flowleap --json tools run get_bibliography patent_number=EP1000000
flowleap --json patent build-query "solid state battery separators" --focus precise
flowleap auth login
flowleap auth create-token --name my-agent --store
```

#### flowleap-uspto — Search USPTO Open Data Portal records with Lucene queries, fetch granted patents, applications and continuity chains, and build ODP queries from natural language.

```bash
flowleap --json uspto search --query 'applicationMetaData.inventionTitle:"machine learning"' --limit 5
flowleap --json uspto build-query "quantum error correction filed after 2022" --focus precise
flowleap --json api request post /v1/patent-search-uspto/search --body '<recommended_query JSON>'
flowleap --json uspto grant 11800000
flowleap --json uspto application 16123456
flowleap --json uspto continuity 16123456
```

### Workflow Triggers

- **persona-ip-analyst** — Persona: IP Analyst — landscape analysis, portfolio assessment, trend mapping.
- **persona-patent-attorney** — Persona: Patent Attorney — search prior art, analyze claims, assess FTO.
- **persona-researcher** — Persona: Researcher — explore patents and academic literature for R&D.
- **persona-startup-founder** — Persona: Startup Founder — validate IP, check FTO, build patent strategy.
- **recipe-academic-literature-review** — Recipe: Academic literature review combined with patent analysis.
- **recipe-claim-analysis** — Recipe: Extract and analyze patent claims with full context.
- **recipe-freedom-to-operate** — Recipe: Freedom-to-operate search for a product or technology.
- **recipe-patent-landscape** — Recipe: Patent landscape analysis for a technology area.
- **recipe-patent-to-report** — Recipe: Extract all data from a patent for structured analysis.
- **recipe-prior-art-search** — Recipe: Comprehensive prior art search across patents and academic literature.
<!-- END FLOWLEAP AGENT RULES -->
