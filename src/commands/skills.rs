use anyhow::{bail, Context as AnyhowContext, Result};
use clap::{Parser, Subcommand};
use include_dir::{include_dir, Dir};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use crate::client::Context;
use crate::output;

/// The skills/ directory is baked into the binary at build time, so
/// `flowleap skills install` works on any machine without the repo.
static EMBEDDED_SKILLS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills");

#[derive(Parser)]
pub struct SkillsArgs {
    #[command(subcommand)]
    command: SkillsCommand,
}

#[derive(Subcommand)]
enum SkillsCommand {
    /// List skills bundled with this CLI
    List,
    /// Install bundled skills into an agent's skills directory
    Install {
        /// Install into the current project's .claude/skills instead of ~/.claude/skills
        #[arg(long, conflicts_with = "dir")]
        project: bool,

        /// Install into an arbitrary directory (for non-Claude agents)
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Overwrite skills that are already installed
        #[arg(long)]
        force: bool,

        /// Only install these skills (default: all)
        #[arg(value_name = "SKILL")]
        names: Vec<String>,
    },
}

pub fn run(ctx: &Context, args: SkillsArgs) -> Result<()> {
    match args.command {
        SkillsCommand::List => list(ctx),
        SkillsCommand::Install {
            project,
            dir,
            force,
            names,
        } => install(ctx, project, dir, force, &names),
    }
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

/// First `description:` line from a skill's SKILL.md frontmatter.
fn skill_description(name: &str) -> Option<String> {
    let file = EMBEDDED_SKILLS.get_file(format!("{}/SKILL.md", name))?;
    let contents = file.contents_utf8()?;
    contents
        .lines()
        .find(|line| line.starts_with("description:"))
        .map(|line| {
            let desc = line.trim_start_matches("description:").trim();
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

fn default_target(project: bool) -> Result<PathBuf> {
    if project {
        return Ok(PathBuf::from(".claude").join("skills"));
    }
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not resolve home directory"))?;
    Ok(home.join(".claude").join("skills"))
}

fn install(
    ctx: &Context,
    project: bool,
    dir: Option<PathBuf>,
    force: bool,
    names: &[String],
) -> Result<()> {
    let target = match dir {
        Some(dir) => dir,
        None => default_target(project)?,
    };

    let available = skill_names();
    let selected: Vec<&str> = if names.is_empty() {
        available.clone()
    } else {
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
        selected
    };

    if ctx.dry_run {
        let value = json!({
            "dryRun": true,
            "target": target,
            "skills": selected,
        });
        output::print_json(&value);
        return Ok(());
    }

    let mut installed: Vec<&str> = Vec::new();
    let mut skipped: Vec<&str> = Vec::new();

    for name in &selected {
        let skill_dir = EMBEDDED_SKILLS
            .get_dir(name)
            .expect("selected skills come from the embedded tree");
        let dest = target.join(name);
        if dest.exists() && !force {
            skipped.push(name);
            continue;
        }
        write_dir(skill_dir, &target)?;
        installed.push(name);
    }

    if ctx.output_format == "json" {
        output::print_json(&json!({
            "ok": true,
            "target": target,
            "installed": installed,
            "skipped": skipped,
            "hint": if skipped.is_empty() { serde_json::Value::Null } else { json!("already installed; use --force to overwrite") },
        }));
    } else {
        println!(
            "Installed {} skill(s) to {}",
            installed.len(),
            target.display()
        );
        for name in &installed {
            println!("  + {}", name);
        }
        if !skipped.is_empty() {
            println!(
                "Skipped {} already-installed skill(s) (use --force to overwrite):",
                skipped.len()
            );
            for name in &skipped {
                println!("  = {}", name);
            }
        }
    }

    Ok(())
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
        let skill = EMBEDDED_SKILLS.get_dir("flowleap").expect("flowleap skill");
        write_dir(skill, &target).expect("write skill");
        assert!(target.join("flowleap/SKILL.md").is_file());
    }
}
