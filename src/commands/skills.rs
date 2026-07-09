use anyhow::{bail, Context as AnyhowContext, Result};
use clap::{Parser, Subcommand, ValueEnum};
use include_dir::{include_dir, Dir};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::client::Context;
use crate::config::{Config, SkillInstall};
use crate::output;

/// The skills/ directory is baked into the binary at build time, so
/// `flowleap skills install` works on any machine without the repo.
static EMBEDDED_SKILLS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills");

/// Markers wrapping the rendered rules block inside shared files
/// (AGENTS.md / GEMINI.md), so installs replace their own block without
/// touching surrounding content. The BEGIN marker carries the version stamp.
const RULES_BEGIN: &str = "<!-- BEGIN FLOWLEAP AGENT RULES";
const RULES_END: &str = "<!-- END FLOWLEAP AGENT RULES -->";

/// Version-stamp marker file written into directory installs (claude / --dir).
const VERSION_STAMP_FILE: &str = ".flowleap-cli-version";

/// Max command lines extracted per skill for the condensed reference.
const COMMANDS_PER_SKILL: usize = 8;

#[derive(Parser)]
pub struct SkillsArgs {
    #[command(subcommand)]
    command: SkillsCommand,
}

/// Harness to install skills for. Claude targets copy the skill directories
/// (Claude Code auto-loads SKILL.md trees); the others render a condensed
/// agent-rules document into the file that harness auto-loads.
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Target {
    /// Copy skill directories into ~/.claude/skills
    Claude,
    /// Copy skill directories into ./.claude/skills
    ClaudeProject,
    /// Render a marked agent-rules block into ./AGENTS.md
    Codex,
    /// Render ./.cursor/rules/flowleap.mdc
    Cursor,
    /// Render a marked agent-rules block into ./GEMINI.md
    Gemini,
}

#[derive(Subcommand)]
enum SkillsCommand {
    /// List skills bundled with this CLI
    List,
    /// Install bundled skills for an agent harness
    Install {
        /// Harness to install for
        #[arg(long, value_enum, default_value_t = Target::Claude, conflicts_with_all = ["project", "dir"])]
        target: Target,

        /// Install into the current project's .claude/skills instead of ~/.claude/skills
        /// (same as --target claude-project)
        #[arg(long, conflicts_with = "dir")]
        project: bool,

        /// Copy skill directories into an arbitrary directory
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Overwrite skills that are already installed
        #[arg(long)]
        force: bool,

        /// Only install these skills (default: all)
        #[arg(value_name = "SKILL")]
        names: Vec<String>,
    },
    /// Re-render/re-copy every recorded install with this CLI's content
    Update,
}

pub fn run(ctx: &Context, args: SkillsArgs) -> Result<()> {
    match args.command {
        SkillsCommand::List => list(ctx),
        SkillsCommand::Install {
            target,
            project,
            dir,
            force,
            names,
        } => install(ctx, target, project, dir, force, &names),
        SkillsCommand::Update => update(ctx),
    }
}

/// Every embedded skill as (name, SKILL.md contents). Shared with the
/// binary's skill↔CLI validation test, which parses each documented
/// `flowleap …` example against the real clap command tree.
pub fn embedded_skill_docs() -> Vec<(&'static str, &'static str)> {
    skill_names()
        .into_iter()
        .filter_map(|name| {
            EMBEDDED_SKILLS
                .get_file(format!("{}/SKILL.md", name))
                .and_then(|file| file.contents_utf8())
                .map(|contents| (name, contents))
        })
        .collect()
}

fn skill_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = EMBEDDED_SKILLS
        .dirs()
        .filter(|d| d.get_file(d.path().join("SKILL.md")).is_some())
        .filter_map(|d| d.path().to_str())
        .collect();
    names.sort_unstable();
    names
}

/// Full `description:` value from a skill's SKILL.md frontmatter, unquoted.
fn skill_description_raw(name: &str) -> Option<String> {
    let file = EMBEDDED_SKILLS.get_file(format!("{}/SKILL.md", name))?;
    let contents = file.contents_utf8()?;
    contents
        .lines()
        .find(|line| line.starts_with("description:"))
        .map(|line| {
            line.trim_start_matches("description:")
                .trim()
                .trim_matches('"')
                .to_string()
        })
}

/// First `description:` line from a skill's SKILL.md frontmatter, shortened
/// for tabular listing.
fn skill_description(name: &str) -> Option<String> {
    skill_description_raw(name).map(|desc| {
        let mut chars = desc.chars();
        let short: String = chars.by_ref().take(100).collect();
        if chars.next().is_some() {
            format!("{}…", short)
        } else {
            short
        }
    })
}

fn list(ctx: &Context) -> Result<()> {
    let skills: Vec<serde_json::Value> = skill_names()
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "description": skill_description(name),
            })
        })
        .collect();

    output::print_value(
        &ctx.output_format,
        &json!(skills),
        &[("name", "Skill"), ("description", "Description")],
    );
    Ok(())
}

fn claude_user_skills_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not resolve home directory"))?;
    Ok(home.join(".claude").join("skills"))
}

/// Resolve the install destination: the record's target string plus the
/// skills directory (copy targets) or rendered file (the others).
fn resolve_destination(
    target: Target,
    project: bool,
    dir: Option<PathBuf>,
) -> Result<(&'static str, PathBuf)> {
    if let Some(dir) = dir {
        return Ok(("dir", dir));
    }
    if project {
        return Ok(("claude-project", PathBuf::from(".claude").join("skills")));
    }
    Ok(match target {
        Target::Claude => ("claude", claude_user_skills_dir()?),
        Target::ClaudeProject => ("claude-project", PathBuf::from(".claude").join("skills")),
        Target::Codex => ("codex", PathBuf::from("AGENTS.md")),
        Target::Cursor => (
            "cursor",
            PathBuf::from(".cursor").join("rules").join("flowleap.mdc"),
        ),
        Target::Gemini => ("gemini", PathBuf::from("GEMINI.md")),
    })
}

/// Copy targets get whole SKILL.md directories; the rest get a rendered file.
fn is_copy_target(target: &str) -> bool {
    matches!(target, "claude" | "claude-project" | "dir")
}

fn install(
    ctx: &Context,
    target: Target,
    project: bool,
    dir: Option<PathBuf>,
    force: bool,
    names: &[String],
) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let (target, path) = resolve_destination(target, project, dir)?;
    let selected = select_skills(names)?;

    if ctx.dry_run {
        let value = json!({
            "dryRun": true,
            "target": target,
            "path": path,
            "skills": selected,
        });
        output::print_json(&value);
        return Ok(());
    }

    let outcome = perform_install(target, &path, &selected, force, version)?;
    record_install(target, &path, names, version)?;

    if ctx.output_format == "json" {
        output::print_json(&json!({
            "ok": true,
            "target": target,
            "path": path,
            "version": version,
            "installed": outcome.installed,
            "skipped": outcome.skipped,
            "hint": if outcome.skipped.is_empty() { serde_json::Value::Null } else { json!("already installed; use --force to overwrite") },
        }));
    } else if is_copy_target(target) {
        println!(
            "Installed {} skill(s) to {}",
            outcome.installed.len(),
            path.display()
        );
        for name in &outcome.installed {
            println!("  + {}", name);
        }
        if !outcome.skipped.is_empty() {
            println!(
                "Skipped {} already-installed skill(s) (use --force to overwrite):",
                outcome.skipped.len()
            );
            for name in &outcome.skipped {
                println!("  = {}", name);
            }
        }
    } else {
        println!(
            "Rendered agent rules for {} skill(s) to {} (flowleap-cli v{})",
            outcome.installed.len(),
            path.display(),
            version
        );
        println!("Refresh after a CLI upgrade with: flowleap skills update");
    }

    Ok(())
}

/// Validate requested skill names against the embedded bundle
/// (empty request = all skills).
fn select_skills(names: &[String]) -> Result<Vec<&'static str>> {
    let available = skill_names();
    if names.is_empty() {
        return Ok(available);
    }
    let mut selected = Vec::new();
    for name in names {
        match available.iter().find(|n| **n == name.as_str()) {
            Some(n) => selected.push(*n),
            None => bail!(
                "Unknown skill: {}. Run 'flowleap skills list' to see bundled skills.",
                name
            ),
        }
    }
    Ok(selected)
}

struct InstallOutcome {
    installed: Vec<String>,
    skipped: Vec<String>,
}

/// Install (or refresh) skills for one resolved target. Copy targets write
/// SKILL.md directories plus a version-stamp file; rendered targets write the
/// condensed agent-rules document (always overwriting their own output).
fn perform_install(
    target: &str,
    path: &Path,
    selected: &[&str],
    force: bool,
    version: &str,
) -> Result<InstallOutcome> {
    if is_copy_target(target) {
        return copy_skills(path, selected, force, version);
    }
    match target {
        "codex" | "gemini" => write_marked_block(path, version, selected)?,
        "cursor" => write_file(path, &render_cursor_mdc(version, selected))?,
        other => bail!("Unknown recorded skills target: {}", other),
    }
    Ok(InstallOutcome {
        installed: selected.iter().map(|s| s.to_string()).collect(),
        skipped: Vec::new(),
    })
}

fn copy_skills(
    target: &Path,
    selected: &[&str],
    force: bool,
    version: &str,
) -> Result<InstallOutcome> {
    let mut installed: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for name in selected {
        let skill_dir = EMBEDDED_SKILLS
            .get_dir(name)
            .expect("selected skills come from the embedded tree");
        let dest = target.join(name);
        if dest.exists() && !force {
            skipped.push(name.to_string());
            continue;
        }
        write_dir(skill_dir, target)?;
        installed.push(name.to_string());
    }

    // Stamp the directory only when we actually wrote current content.
    if !installed.is_empty() {
        write_file(&target.join(VERSION_STAMP_FILE), &format!("{}\n", version))?;
    }

    Ok(InstallOutcome { installed, skipped })
}

/// Upsert the rendered rules block into a shared file (AGENTS.md / GEMINI.md),
/// preserving all surrounding content.
fn write_marked_block(path: &Path, version: &str, selected: &[&str]) -> Result<()> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let block = render_agents_block(version, selected);
    write_file(path, &upsert_marked_block(&existing, &block))
}

/// Replace the existing marked block, or append one to the end of the file.
fn upsert_marked_block(existing: &str, block: &str) -> String {
    let bounds = existing.find(RULES_BEGIN).and_then(|start| {
        existing[start..]
            .find(RULES_END)
            .map(|end| (start, start + end + RULES_END.len()))
    });
    match bounds {
        Some((start, end)) => format!(
            "{}{}{}",
            &existing[..start],
            block.trim_end(),
            &existing[end..]
        ),
        None => {
            let head = existing.trim_end();
            if head.is_empty() {
                format!("{}\n", block.trim_end())
            } else {
                format!("{}\n\n{}\n", head, block.trim_end())
            }
        }
    }
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create directory {}", parent.display()))?;
        }
    }
    fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

/// Upsert this install into config.toml so `skills update` can refresh it.
fn record_install(target: &str, path: &Path, names: &[String], version: &str) -> Result<()> {
    let mut cfg = Config::load()?;
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    match cfg
        .skill_installs
        .iter_mut()
        .find(|rec| rec.target == target && rec.path == path)
    {
        Some(rec) => {
            rec.version = version.to_string();
            rec.skills = names.to_vec();
        }
        None => cfg.skill_installs.push(SkillInstall {
            target: target.to_string(),
            path,
            version: version.to_string(),
            skills: names.to_vec(),
        }),
    }
    cfg.save()
}

fn update(ctx: &Context) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let mut cfg = Config::load()?;

    if cfg.skill_installs.is_empty() {
        if ctx.output_format == "json" {
            output::print_json(&json!({ "ok": true, "updated": [] }));
        } else {
            println!("No recorded skill installs. Run 'flowleap skills install' first.");
        }
        return Ok(());
    }

    if ctx.dry_run {
        output::print_json(&json!({
            "dryRun": true,
            "version": version,
            "targets": cfg.skill_installs,
        }));
        return Ok(());
    }

    apply_updates(&mut cfg.skill_installs, version)?;
    let updated = cfg.skill_installs.clone();
    cfg.save()?;

    if ctx.output_format == "json" {
        output::print_json(&json!({
            "ok": true,
            "version": version,
            "updated": updated,
        }));
    } else {
        println!(
            "Refreshed {} skill install(s) to flowleap-cli v{}",
            updated.len(),
            version
        );
        for rec in &updated {
            println!("  * {} -> {}", rec.target, rec.path.display());
        }
    }
    Ok(())
}

/// Re-render/re-copy every recorded install with this binary's embedded
/// content and bump each record's stamp. Pure over the record list so tests
/// can drive it against temporary paths.
fn apply_updates(installs: &mut [SkillInstall], version: &str) -> Result<()> {
    let available = skill_names();
    for rec in installs.iter_mut() {
        // Skills recorded under an older CLI may since have been renamed or
        // dropped from the bundle; refresh the ones that still exist.
        let selected: Vec<&str> = if rec.skills.is_empty() {
            available.clone()
        } else {
            available
                .iter()
                .filter(|name| rec.skills.iter().any(|s| s == **name))
                .copied()
                .collect()
        };
        perform_install(&rec.target, &rec.path, &selected, true, version)?;
        rec.version = version.to_string();
    }
    Ok(())
}

/// Line for the daily update notice (hooked from src/update.rs): mentions
/// recorded skill installs whose stamp is older than the running binary.
pub fn stale_skills_notice(current_version: &str) -> Option<String> {
    let cfg = Config::load().ok()?;
    stale_notice_for(&cfg.skill_installs, current_version)
}

fn stale_notice_for(installs: &[SkillInstall], current_version: &str) -> Option<String> {
    let stale = installs
        .iter()
        .filter(|rec| crate::update::is_newer(current_version, &rec.version))
        .count();
    (stale > 0).then(|| {
        format!(
            "{} installed skill target(s) were rendered by an older flowleap. Refresh: flowleap skills update",
            stale
        )
    })
}

// ---------------------------------------------------------------------------
// Rules renderer: one condensed agent-rules document generated from the
// embedded skill content, shared by every non-Claude target.
// ---------------------------------------------------------------------------

/// Persona/recipe skills contribute workflow triggers; everything else
/// contributes to the command reference.
fn is_workflow_skill(name: &str) -> bool {
    name.starts_with("persona-") || name.starts_with("recipe-")
}

/// First sentence of a skill description, for compact reference headings.
fn first_sentence(desc: &str) -> &str {
    match desc.find(". ") {
        Some(ix) => &desc[..ix + 1],
        None => desc,
    }
}

/// Extract up to `cap` representative `flowleap …` command lines from a
/// skill's fenced code blocks, deduplicated by command/subcommand signature.
fn skill_commands(name: &str, cap: usize) -> Vec<String> {
    let Some(contents) = EMBEDDED_SKILLS
        .get_file(format!("{}/SKILL.md", name))
        .and_then(|file| file.contents_utf8())
    else {
        return Vec::new();
    };

    let mut commands = Vec::new();
    let mut seen = HashSet::new();
    let mut in_fence = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence || !trimmed.starts_with("flowleap ") {
            continue;
        }
        let command = trimmed
            .split_once(" #")
            .map_or(trimmed, |(cmd, _)| cmd)
            .trim_end();
        // Signature: the first non-flag, non-placeholder tokens identify the
        // subcommand, so variations on flags collapse to one line.
        let signature: Vec<&str> = command
            .split_whitespace()
            .filter(|tok| !tok.starts_with('-') && !tok.starts_with('<') && !tok.starts_with('"'))
            .take(3)
            .collect();
        if seen.insert(signature.join(" ")) {
            commands.push(command.to_string());
        }
        if commands.len() >= cap {
            break;
        }
    }
    commands
}

/// Render the condensed agent-rules markdown body (no target wrapper):
/// ground rules, a command reference from service skills, and workflow
/// trigger summaries from persona/recipe skills.
fn render_rules_body(version: &str, selected: &[&str]) -> String {
    let mut out = String::new();
    out.push_str("## FlowLeap CLI — Agent Rules\n\n");
    out.push_str(&format!(
        "Rendered by flowleap-cli v{} from its bundled agent skills. Refresh after a CLI upgrade with `flowleap skills update`.\n\n",
        version
    ));
    out.push_str(
        "FlowLeap is a CLI for the FlowLeap Patent AI backend: patent/USPTO/OPS/academic/NPL/legal/citation search plus an agent-first tools facade.\n\n\
         - Add `--json` to every command for stable machine-readable output.\n\
         - Run `flowleap --json doctor` first to verify config, auth, and backend reachability.\n\
         - Authenticate with `flowleap auth login`, or set `FLOWLEAP_API_KEY`/`FLOWLEAP_TOKEN` for headless use.\n\
         - If an error carries a `providerKeysHint` (`provider_keys_required` / `provider_keys_invalid`), stop — this needs a human. Ask the user to run `flowleap setup`; do not retry or invent keys.\n\
         - Discover every backend tool with `flowleap tools list`; run one with `flowleap tools run <name>`.\n\n",
    );

    out.push_str("### Command Reference\n\n");
    for name in selected.iter().filter(|n| !is_workflow_skill(n)) {
        let desc = skill_description_raw(name).unwrap_or_default();
        out.push_str(&format!("#### {} — {}\n\n", name, first_sentence(&desc)));
        let commands = skill_commands(name, COMMANDS_PER_SKILL);
        if !commands.is_empty() {
            out.push_str("```bash\n");
            for command in &commands {
                out.push_str(command);
                out.push('\n');
            }
            out.push_str("```\n\n");
        }
    }

    let workflows: Vec<&&str> = selected.iter().filter(|n| is_workflow_skill(n)).collect();
    if !workflows.is_empty() {
        out.push_str("### Workflow Triggers\n\n");
        for name in workflows {
            let desc = skill_description_raw(name).unwrap_or_default();
            out.push_str(&format!("- **{}** — {}\n", name, desc));
        }
    }
    out
}

/// The rules body wrapped in version-stamped markers, for shared files
/// (AGENTS.md for Codex, GEMINI.md for Gemini CLI).
fn render_agents_block(version: &str, selected: &[&str]) -> String {
    format!(
        "{} (flowleap-cli v{}) -->\n{}{}\n",
        RULES_BEGIN,
        version,
        render_rules_body(version, selected),
        RULES_END
    )
}

/// The rules body as a Cursor project rule (.cursor/rules/flowleap.mdc),
/// with MDC frontmatter so Cursor auto-attaches it.
fn render_cursor_mdc(version: &str, selected: &[&str]) -> String {
    format!(
        "---\ndescription: FlowLeap Patent AI CLI — command reference and workflow triggers for agents\nalwaysApply: true\n---\n<!-- flowleap-cli v{} — refresh with `flowleap skills update` -->\n\n{}",
        version,
        render_rules_body(version, selected)
    )
}

/// Recursively write an embedded directory under `target` (paths in the
/// embedded tree are relative to skills/, so files land at target/<skill>/…).
fn write_dir(dir: &Dir<'_>, target: &Path) -> Result<()> {
    for entry_dir in dir.dirs() {
        write_dir(entry_dir, target)?;
    }
    for file in dir.files() {
        let dest = target.join(file.path());
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create directory {}", parent.display()))?;
        }
        fs::write(&dest, file.contents()).with_context(|| format!("write {}", dest.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_VERSION: &str = "0.0.0-test";

    #[test]
    fn embedded_skills_present() {
        let names = skill_names();
        assert!(
            names.contains(&"flowleap"),
            "core flowleap skill must be embedded, got: {:?}",
            names
        );
        for name in &names {
            assert!(
                skill_description(name).is_some(),
                "skill {} must have a description in frontmatter",
                name
            );
        }
    }

    #[test]
    fn install_and_skip_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("skills");
        let outcome = copy_skills(&target, &["flowleap"], false, TEST_VERSION).expect("install");
        assert!(target.join("flowleap/SKILL.md").is_file());
        assert_eq!(
            fs::read_to_string(target.join(VERSION_STAMP_FILE)).expect("stamp"),
            format!("{}\n", TEST_VERSION)
        );
        assert_eq!(outcome.installed, vec!["flowleap"]);

        // Second install without --force skips and leaves the stamp alone.
        let outcome = copy_skills(&target, &["flowleap"], false, "9.9.9").expect("reinstall");
        assert_eq!(outcome.skipped, vec!["flowleap"]);
        assert_eq!(
            fs::read_to_string(target.join(VERSION_STAMP_FILE)).expect("stamp"),
            format!("{}\n", TEST_VERSION)
        );
    }

    /// Compare rendered output against a golden file; run with
    /// UPDATE_GOLDEN=1 to (re)generate the goldens after content changes.
    fn assert_golden(got: &str, golden: &str) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(golden);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, got).unwrap();
        }
        let want = fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!("missing golden {}; run UPDATE_GOLDEN=1 cargo test", golden)
        });
        assert_eq!(got, want, "golden mismatch: {}", golden);
    }

    #[test]
    fn golden_codex_agents_block() {
        assert_golden(
            &render_agents_block(TEST_VERSION, &skill_names()),
            "agents-rules-block.md",
        );
    }

    #[test]
    fn golden_gemini_block_matches_codex() {
        // Codex and Gemini both auto-load a marked markdown block; the
        // rendered output is identical by design.
        assert_golden(
            &render_agents_block(TEST_VERSION, &skill_names()),
            "agents-rules-block.md",
        );
    }

    #[test]
    fn golden_cursor_rules() {
        assert_golden(
            &render_cursor_mdc(TEST_VERSION, &skill_names()),
            "cursor-flowleap.mdc",
        );
    }

    #[test]
    fn rendered_output_carries_version_stamp() {
        let names = skill_names();
        assert!(render_agents_block("1.2.3", &names).contains("flowleap-cli v1.2.3"));
        assert!(render_cursor_mdc("1.2.3", &names).contains("flowleap-cli v1.2.3"));
    }

    #[test]
    fn upsert_replaces_only_the_marked_block() {
        let names = skill_names();
        let old_block = render_agents_block("0.0.1", &names);
        let existing = format!(
            "# My Project\n\nHand-written rules stay.\n\n{}\nTrailing notes stay too.\n",
            old_block.trim_end()
        );
        let new_block = render_agents_block("0.0.2", &names);
        let merged = upsert_marked_block(&existing, &new_block);
        assert!(merged.contains("# My Project"));
        assert!(merged.contains("Hand-written rules stay."));
        assert!(merged.contains("Trailing notes stay too."));
        assert!(merged.contains("flowleap-cli v0.0.2"));
        assert!(!merged.contains("flowleap-cli v0.0.1"));
        assert_eq!(merged.matches(RULES_BEGIN).count(), 1);
    }

    #[test]
    fn upsert_appends_when_no_block_present() {
        let block = render_agents_block(TEST_VERSION, &["flowleap"]);
        let merged = upsert_marked_block("# Existing\n", &block);
        assert!(merged.starts_with("# Existing\n\n"));
        assert!(merged.trim_end().ends_with(RULES_END));

        let fresh = upsert_marked_block("", &block);
        assert!(fresh.starts_with(RULES_BEGIN));
    }

    #[test]
    fn record_then_update_refreshes_every_target() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut installs = vec![
            SkillInstall {
                target: "codex".to_string(),
                path: tmp.path().join("AGENTS.md"),
                version: "0.0.1".to_string(),
                skills: Vec::new(),
            },
            SkillInstall {
                target: "cursor".to_string(),
                path: tmp.path().join(".cursor/rules/flowleap.mdc"),
                version: "0.0.1".to_string(),
                skills: Vec::new(),
            },
            SkillInstall {
                target: "gemini".to_string(),
                path: tmp.path().join("GEMINI.md"),
                version: "0.0.1".to_string(),
                skills: Vec::new(),
            },
            SkillInstall {
                target: "dir".to_string(),
                path: tmp.path().join("skills"),
                version: "0.0.1".to_string(),
                skills: vec!["flowleap".to_string(), "no-longer-bundled".to_string()],
            },
        ];

        apply_updates(&mut installs, "9.9.9").expect("update");

        for rec in &installs {
            assert_eq!(rec.version, "9.9.9", "record stamp for {}", rec.target);
        }
        for file in ["AGENTS.md", ".cursor/rules/flowleap.mdc", "GEMINI.md"] {
            let contents = fs::read_to_string(tmp.path().join(file)).expect(file);
            assert!(
                contents.contains("flowleap-cli v9.9.9"),
                "{} must carry the new stamp",
                file
            );
        }
        assert!(tmp.path().join("skills/flowleap/SKILL.md").is_file());
        assert_eq!(
            fs::read_to_string(tmp.path().join("skills").join(VERSION_STAMP_FILE)).expect("stamp"),
            "9.9.9\n"
        );
    }

    #[test]
    fn stale_notice_fires_only_for_older_stamps() {
        let rec = |version: &str| SkillInstall {
            target: "codex".to_string(),
            path: PathBuf::from("AGENTS.md"),
            version: version.to_string(),
            skills: Vec::new(),
        };
        assert!(stale_notice_for(&[rec("0.2.4")], "0.2.5").is_some());
        assert!(stale_notice_for(&[rec("0.2.5")], "0.2.5").is_none());
        assert!(stale_notice_for(&[], "0.2.5").is_none());
        let notice = stale_notice_for(&[rec("0.1.0"), rec("0.2.0")], "0.2.5").expect("notice");
        assert!(notice.contains("2 installed skill target(s)"));
        assert!(notice.contains("flowleap skills update"));
    }

    #[test]
    fn command_extraction_dedupes_by_subcommand() {
        let commands = skill_commands("flowleap-patent", COMMANDS_PER_SKILL);
        assert!(
            commands.iter().any(|c| c.contains("patent search")),
            "expected a patent search example, got: {:?}",
            commands
        );
        assert!(commands.len() <= COMMANDS_PER_SKILL);
    }
}
