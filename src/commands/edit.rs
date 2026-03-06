use anyhow::{Result, bail};
use console::{Term, style};

use crate::commands::init;
use crate::config::Paths;
use crate::manifest::Manifest;
use crate::plugins;
use crate::project;
use crate::symlinks;

pub fn run() -> Result<()> {
    let paths = Paths::resolve()?;
    let term = Term::stderr();
    let project_dir = std::env::current_dir()?;

    let Some(existing) = Manifest::read(&project_dir)? else {
        bail!(
            "No .claude/ccpick.json found in {}. \
             Run `ccpick init` first.",
            project_dir.display(),
        );
    };

    let commands = init::pick_category(
        &term,
        "commands",
        &paths.library_commands(),
        Some(&existing.commands),
    )?;

    let skills = init::pick_category(
        &term,
        "skills",
        &paths.library_skills(),
        Some(&existing.skills),
    )?;

    let plugin_infos = plugins::scan_plugins(&paths.claude_home)?;
    let plugin_map = init::pick_plugins(
        &term,
        &plugin_infos,
        Some(&existing.plugins),
        "Select plugins to enable",
    )?;

    symlinks::apply(&project_dir, &paths, &commands, &skills)?;

    let settings_path = project_dir.join(".claude").join("settings.local.json");
    project::merge_enabled_plugins(&settings_path, &plugin_map)?;

    let manifest = Manifest {
        version: 1,
        commands,
        skills,
        plugins: plugin_map,
    };
    manifest.write(&project_dir)?;

    term.write_line(&format!(
        "\n{} Manifest updated: .claude/ccpick.json",
        style("✓").green(),
    ))?;

    Ok(())
}
