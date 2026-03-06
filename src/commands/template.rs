use anyhow::{Result, bail};
use console::{Term, style};
use dialoguer::Input;

use crate::cli::TemplateAction;
use crate::commands::init;
use crate::config::Paths;
use crate::manifest::Manifest;
use crate::plugins;

pub fn run(action: &TemplateAction) -> Result<()> {
    let paths = Paths::resolve()?;
    match action {
        TemplateAction::Save { name } => save(&paths, name),
        TemplateAction::Create { name } => create(&paths, name.as_deref()),
        TemplateAction::Apply { name } => apply(&paths, name),
        TemplateAction::Edit { name } => edit(&paths, name),
        TemplateAction::List => list(&paths),
        TemplateAction::Delete { name } => delete(&paths, name),
    }
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("template name cannot be empty");
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!(
            "template name may only contain \
             alphanumeric characters, hyphens, \
             and underscores"
        );
    }
    Ok(())
}

fn prompt_name(default: Option<&str>) -> Result<String> {
    let input = Input::<String>::new().with_prompt("Template name");
    let input = match default {
        Some(d) => input.default(d.to_string()),
        None => input,
    };
    input
        .validate_with(|s: &String| -> Result<(), String> {
            validate_name(s).map_err(|e| e.to_string())
        })
        .interact_text()
        .map_err(Into::into)
}

fn save(paths: &Paths, name: &str) -> Result<()> {
    validate_name(name)?;
    let term = Term::stderr();
    let project_dir = std::env::current_dir()?;

    let Some(manifest) = Manifest::read(&project_dir)? else {
        bail!(
            "No .claude/ccpick.json found in {}. \
             Run `ccpick init` first.",
            project_dir.display(),
        );
    };

    let templates_dir = paths.templates_dir();
    manifest.write_template(&templates_dir, name)?;

    term.write_line(&format!("{} Template '{name}' saved.", style("✓").green(),))?;
    Ok(())
}

fn create(paths: &Paths, name: Option<&str>) -> Result<()> {
    if let Some(n) = name {
        validate_name(n)?;
    }
    let term = Term::stderr();

    let commands = init::pick_category(&term, "commands", &paths.library_commands(), None)?;

    let skills = init::pick_category(&term, "skills", &paths.library_skills(), None)?;

    let plugin_infos = plugins::scan_plugins(&paths.claude_home)?;
    let plugin_map = init::pick_plugins(&term, &plugin_infos, None, "Select plugins to enable")?;

    let final_name = prompt_name(name)?;

    let manifest = Manifest {
        version: 1,
        commands,
        skills,
        plugins: plugin_map,
    };
    let templates_dir = paths.templates_dir();
    manifest.write_template(&templates_dir, &final_name)?;

    term.write_line(&format!(
        "\n{} Template '{final_name}' created.",
        style("✓").green(),
    ))?;
    Ok(())
}

fn apply(paths: &Paths, name: &str) -> Result<()> {
    validate_name(name)?;
    let term = Term::stderr();
    let project_dir = std::env::current_dir()?;
    init::apply_template(&term, paths, &project_dir, name)
}

fn edit(paths: &Paths, name: &str) -> Result<()> {
    validate_name(name)?;
    let term = Term::stderr();
    let templates_dir = paths.templates_dir();

    let existing = Manifest::read_template(&templates_dir, name)?;

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

    let manifest = Manifest {
        version: 1,
        commands,
        skills,
        plugins: plugin_map,
    };
    manifest.write_template(&templates_dir, name)?;

    term.write_line(&format!(
        "\n{} Template '{name}' updated.",
        style("✓").green(),
    ))?;
    Ok(())
}

fn list(paths: &Paths) -> Result<()> {
    let term = Term::stderr();
    let templates_dir = paths.templates_dir();
    let names = Manifest::list_templates(&templates_dir)?;

    if names.is_empty() {
        term.write_line("No templates saved.")?;
        return Ok(());
    }

    for name in &names {
        term.write_line(name)?;
    }
    Ok(())
}

fn delete(paths: &Paths, name: &str) -> Result<()> {
    validate_name(name)?;
    let term = Term::stderr();
    let templates_dir = paths.templates_dir();
    Manifest::delete_template(&templates_dir, name)?;

    term.write_line(&format!(
        "{} Template '{name}' deleted.",
        style("✓").green(),
    ))?;
    Ok(())
}
