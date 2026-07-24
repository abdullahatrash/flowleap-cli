# FlowLeap CLI

Rust CLI for the FlowLeap Patent AI backend, and the canonical home of the
FlowLeap agent skills.

## Language

**CLI skill**:
A SKILL.md in this repo's `skills/` directory, written in the CLI dialect —
its instructions invoke `flowleap …` commands. Canonical: this is where
capability-skills are authored first. Baked into the binary at build time and
installed via `flowleap skills install/update`.
_Avoid_: calling these just "skills" when the app dialect could be meant.

**App skill**:
A SKILL.md in `flowleap-agent-v2 …/assets/skills/`, written in the VS Code
extension's tool dialect (`get_patent_summary`, `patent_api_request`, …) with
`user-invocable` frontmatter. Maintained separately — there is NO sync between
CLI skills and app skills; overlapping workflows (e.g. office-action response)
exist in both dialects and drift independently.

**Skill Pack**:
The marketplace distribution unit: a plugin in the `flowleap-plugins` monorepo
containing CLI skills copied byte-for-byte from a pinned flowleap-cli tag
(`sync.json` ref, drift-checked in CI). The website marketplace renders its
catalog from Skill Packs at build time. Skill Packs ship CLI skills only —
app skills never flow through them.

**Capability vs. skill**:
Skills instruct, tools reach. A *capability* (data access — a backend
endpoint/tool) is what agents call; a *skill* is instructions composing
existing capabilities into a workflow. A skill cannot substitute for a missing
capability, and a capability without a skill is undiscoverable in practice.
See AGENTS.md "Skills vs. tools" for the authoring policy.
