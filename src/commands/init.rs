use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use console::{Term, style};
use dialoguer::{Confirm, MultiSelect};

use crate::cli::InitArgs;
use crate::config::Paths;
use crate::manifest::Manifest;
use crate::plugins;
use crate::project;
use crate::scanner;
use crate::symlinks;

pub fn run(args: &InitArgs) -> Result<()> {
    let paths = Paths::resolve()?;
    let term = Term::stderr();
    let project_dir = std::env::current_dir()?;

    if let Some(name) = &args.template {
        return apply_template(&term, &paths, &project_dir, name);
    }

    interactive_init(&term, &paths, &project_dir)
}

pub(crate) fn apply_template(
    term: &Term,
    paths: &Paths,
    project_dir: &Path,
    name: &str,
) -> Result<()> {
    let manifest = Manifest::read_template(&paths.templates_dir(), name)?;

    ensure_claude_dir(project_dir)?;

    symlinks::apply(project_dir, paths, &manifest.commands, &manifest.skills)?;

    let settings_path = project_dir.join(".claude").join("settings.local.json");
    project::merge_enabled_plugins(&settings_path, &manifest.plugins)?;

    manifest.write(project_dir)?;

    term.write_line(&format!(
        "{} Applied template '{name}'. \
         Manifest saved to .claude/ccpick.json",
        style("✓").green(),
    ))?;

    Ok(())
}

fn ensure_claude_dir(project_dir: &Path) -> Result<()> {
    let claude_dir = project_dir.join(".claude");
    std::fs::create_dir_all(&claude_dir)
        .map_err(|e| anyhow::Error::new(e).context(format!("creating {}", claude_dir.display())))?;
    Ok(())
}

fn interactive_init(term: &Term, paths: &Paths, project_dir: &Path) -> Result<()> {
    if !project_dir.join(".claude").is_dir() {
        let Some(true) = Confirm::new()
            .with_prompt(format!(
                "No .claude/ directory in {}. Create it?",
                project_dir.display(),
            ))
            .default(true)
            .interact_opt()?
        else {
            return Ok(());
        };
    }

    let existing = Manifest::read(project_dir)?;

    let commands = pick_category(
        term,
        "commands",
        &paths.library_commands(),
        existing.as_ref().map(|m| &m.commands),
    )?;

    let skills = pick_category(
        term,
        "skills",
        &paths.library_skills(),
        existing.as_ref().map(|m| &m.skills),
    )?;

    let plugin_infos = plugins::scan_plugins(&paths.claude_home)?;
    let plugin_defaults = match existing.as_ref() {
        Some(m) => m.plugins.clone(),
        None => project::read_global_enabled_plugins(paths)?,
    };
    let plugin_map = pick_plugins(
        term,
        &plugin_infos,
        Some(&plugin_defaults),
        "Select plugins to enable",
    )?;

    symlinks::apply(project_dir, paths, &commands, &skills)?;

    let settings_path = project_dir.join(".claude").join("settings.local.json");
    project::merge_enabled_plugins(&settings_path, &plugin_map)?;

    let manifest = Manifest {
        version: 1,
        commands,
        skills,
        plugins: plugin_map,
    };
    manifest.write(project_dir)?;

    term.write_line(&format!(
        "\n{} Project configured. Manifest saved to \
         .claude/ccpick.json",
        style("✓").green(),
    ))?;

    Ok(())
}

pub(crate) fn pick_category(
    term: &Term,
    category: &str,
    library_dir: &Path,
    existing: Option<&Vec<String>>,
) -> Result<Vec<String>> {
    let available = if category == "skills" {
        scanner::scan_skill_dirs(library_dir)?
    } else {
        scanner::scan_md_files(library_dir)?
    };

    if available.is_empty() {
        term.write_line(&format!("  No {category} in ccpick library, skipping.",))?;
        return Ok(Vec::new());
    }

    let labels: Vec<String> = available.iter().map(|p| p.display().to_string()).collect();

    let defaults: Vec<bool> = available
        .iter()
        .map(|p| {
            let s = p.display().to_string();
            existing.is_some_and(|list| list.contains(&s))
        })
        .collect();

    term.write_line(&format!(
        "\nSelect {category} to enable \
         (space to toggle, enter to confirm):",
    ))?;

    let Some(selected) = MultiSelect::new()
        .items(&labels)
        .defaults(&defaults)
        .interact_opt()?
    else {
        return Err(crate::UserAbort.into());
    };

    Ok(selected.into_iter().map(|i| labels[i].clone()).collect())
}

pub(crate) fn pick_plugins(
    term: &Term,
    plugin_infos: &[plugins::PluginInfo],
    existing: Option<&BTreeMap<String, bool>>,
    prompt: &str,
) -> Result<BTreeMap<String, bool>> {
    if plugin_infos.is_empty() {
        term.write_line("  No plugins installed, skipping.")?;
        return Ok(BTreeMap::new());
    }

    let labels: Vec<String> = plugin_infos
        .iter()
        .map(plugins::PluginInfo::label)
        .collect();

    let defaults: Vec<bool> = plugin_infos
        .iter()
        .map(|p| existing.and_then(|m| m.get(&p.id)).copied().unwrap_or(true))
        .collect();

    term.write_line(&format!("\n{prompt} (space to toggle, enter to confirm):",))?;

    let Some(selected) = MultiSelect::new()
        .items(&labels)
        .defaults(&defaults)
        .interact_opt()?
    else {
        return Err(crate::UserAbort.into());
    };

    let mut map = BTreeMap::new();
    for (idx, info) in plugin_infos.iter().enumerate() {
        map.insert(info.id.clone(), selected.contains(&idx));
    }
    Ok(map)
}
