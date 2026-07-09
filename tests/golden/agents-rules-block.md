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

#### flowleap — Start here — the umbrella skill for the FlowLeap Patent AI CLI.

```bash
flowleap --json doctor
flowleap --json doctor --base-url http://localhost:8000
flowleap --json health --base-url http://localhost:8000
flowleap --json health cache --base-url http://localhost:8000
flowleap --json doctor --base-url https://api.flowleap.co
flowleap auth login --api-key fl_pat_your_token_here
flowleap --json api profile
flowleap auth create-token --name my-agent --store
```

#### flowleap-academic — Search academic literature (Semantic Scholar + arXiv) through the FlowLeap backend with per-source and publication-year filters.

```bash
flowleap academic search <query> [flags]
```

#### flowleap-auth — Authenticate the FlowLeap CLI — OAuth 2.0 device flow login (user code + verification URL), long-lived fl_pat_ personal API tokens for headless agents, status checks, and targeted logout.

```bash
flowleap auth login
flowleap auth create-token --name my-agent --store
flowleap auth tokens
flowleap auth revoke-token <id>
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

#### flowleap-ops — Direct EPO Open Patent Services access through the FlowLeap backend — CQL search plus per-document bibliography, claims, description, family, legal status, and abstract.

```bash
flowleap ops search --cql <query> [flags]
flowleap ops biblio <doc>
flowleap ops claims <doc> --lang en
flowleap ops description <doc> --lang en
flowleap ops family <doc>
flowleap ops legal <doc>
flowleap ops abstract <doc>
```

#### flowleap-patent — Search EPO patents with CQL and build CQL queries from natural language through the FlowLeap backend.

```bash
flowleap patent search --query <query> [flags]
flowleap uspto search --query "lithium battery" --limit 20
flowleap patent build-query <description> [flags]
```

#### flowleap-shared — Shared reference for every FlowLeap skill — authentication (OAuth device flow, fl_pat_ personal API tokens), credential storage, config precedence, global flags, and output formats.

```bash
flowleap auth login
flowleap auth create-token --name my-agent --store
flowleap auth status
flowleap auth logout
flowleap config set base-url https://api.flowleap.co
flowleap config get base-url
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

- **persona-ip-analyst** — Persona bundle for IP-analyst workflows with the FlowLeap CLI — technology landscape mapping, portfolio assessment, white-space identification, and filing-trend analytics. Trigger when the user asks the agent to act as an IP analyst or requests landscape mapping, competitor portfolio analysis, or patent filing trend reports.
- **persona-patent-attorney** — Persona bundle for patent-attorney workflows with the FlowLeap CLI — prior art searching, claim reading, legal-status checks, and freedom-to-operate assessment. Trigger when the user asks the agent to act as a patent attorney or requests attorney-grade prior art review, claim scope analysis, or FTO clearance work.
- **persona-researcher** — Persona bundle for R&D-researcher workflows with the FlowLeap CLI — parallel literature and patent exploration to map what is published versus what is protected. Trigger when the user asks the agent to act as a researcher, survey a technology area across papers and patents, or find gaps between academic work and filed IP.
- **persona-startup-founder** — Persona bundle for startup-founder IP workflows with the FlowLeap CLI — novelty sanity checks, freedom-to-operate scans, competitor patent monitoring, and patent strategy grounding. Trigger when the user asks the agent to help validate a startup's IP position, check whether an idea is already patented, or scope competitor patents before building.
- **recipe-academic-literature-review** — Recipe for a technology review combining academic literature and patent data — scholarly search across Semantic Scholar, arXiv, and OpenAlex plus a matching patent sweep. Trigger when the user asks for a literature review, a state-of-the-art survey, or a comparison of published research against filed patents.
- **recipe-audit-report** — Governance recipe for producing an auditable record of AI-assisted patent research — reproducible command log, data provenance for every finding, portfolio status verification, and an AI-usage disclosure section suitable for internal or filing-adjacent records. Trigger when the user asks for an audit trail of patent research, an AI-assistance disclosure, or a verifiable methodology write-up of analysis work.
- **recipe-claim-analysis** — Recipe for extracting and analyzing a patent's claims with full context — claims text, abstract, bibliography, description, family, plus automated element decomposition. Trigger when the user asks to analyze what a patent claims, break claims into elements, or interpret claim scope with supporting context.
- **recipe-claim-drafting** — Prosecution recipe for drafting patent claims grounded in prior art — search the closest art, study its claim language, decompose the invention into elements, iterate drafts through claim analysis, and check formal drafting rules against MPEP/EPO guidelines. Trigger when the user asks to draft or improve patent claims, write independent and dependent claims, or stress-test draft claims against known art and formal requirements.
- **recipe-freedom-to-operate** — Recipe for a freedom-to-operate search — per-feature query generation, dual-database blocking-patent search, and legal-status, family, and claims checks on every candidate. Trigger when the user asks whether a product or technology can be launched without infringing, or requests an FTO/clearance search.
- **recipe-infringement-charting** — Litigation recipe for an element-by-element infringement claim chart — verify the patent is in force where it matters, decompose the asserted claims, extract accused-product evidence (OCR datasheets and manuals), and map every element with literal and doctrine-of-equivalents columns. Trigger when the user asks whether a product infringes a patent, wants a claim chart, or needs element-by-element infringement mapping.
- **recipe-invalidity-analysis** — Litigation recipe for building an invalidity case against a target patent — pin the priority date, decompose claims into elements, hunt patent and non-patent prior art per element, and assemble an invalidity chart with X/Y/A reference tagging (single-reference novelty vs. combination obviousness vs. background). Trigger when the user asks to invalidate a patent, find 102/103 or Art. 54/56 prior art against granted claims, or assess how strong a patent really is.
- **recipe-invention-disclosure** — Prosecution recipe for turning an inventor conversation into a complete invention disclosure form (IDF) — structured technical capture, bar-date audit, novelty pre-check across patents and literature, landscape context, and a filing recommendation. Trigger when the user asks to document an invention, create an invention disclosure, or run a pre-filing novelty sanity check.
- **recipe-office-action-response** — Prosecution recipe for turning an office action into a structured draft response — OCR the OA, pull every cited reference's claims and bibliography, map rejections element-by-element, and ground arguments in MPEP/EPO guideline citations. Trigger when the user asks to respond to an office action, analyze examiner rejections, or prepare arguments against novelty/obviousness (102/103, Art. 54/56) objections.
- **recipe-patent-landscape** — Recipe for patent landscape analysis of a technology area — scoped searches, key-player identification, recent-activity checks, and full-corpus filing analytics. Trigger when the user asks to map a technology space, identify who patents in an area, or report filing trends and white space.
- **recipe-patent-to-report** — Recipe for extracting everything about one patent into a structured report — bibliography, abstract, claims, description, family, legal status, prosecution timeline, figures, and related art. Trigger when the user asks for a complete profile, dossier, or report on a specific patent.
- **recipe-prior-art-search** — Recipe for a comprehensive prior art search — natural-language query generation, dual EPO/USPTO patent search, academic literature sweep, and deep dives on the closest hits. Trigger when the user asks to find prior art for an invention, run a novelty search, or check what already exists before filing.
<!-- END FLOWLEAP AGENT RULES -->
