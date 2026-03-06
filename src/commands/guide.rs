use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;

use anyhow::{Result, bail};
use console::{Term, style};
use dialoguer::{Confirm, Select};

use crate::cli::{GuideAction, GuidePresetAction, GuidePresetCreateArgs, GuideTemplateAction};
use crate::config::Paths;
use crate::guide;

pub fn run(action: &GuideAction) -> Result<()> {
    let paths = Paths::resolve()?;
    let guide_dir = paths.guide_dir();

    match action {
        GuideAction::Template { action } => run_template(&guide_dir, action),
        GuideAction::Preset { action } => run_preset(&guide_dir, action),
        GuideAction::Apply { name } => apply(&guide_dir, name.as_deref()),
        GuideAction::Compose { name } => compose(&guide_dir, name.as_deref()),
        GuideAction::Show { name } => show(&guide_dir, name),
    }
}

// ---------------------------------------------------------------------------
// Template subcommands
// ---------------------------------------------------------------------------

fn run_template(guide_dir: &Path, action: &GuideTemplateAction) -> Result<()> {
    match action {
        GuideTemplateAction::List => template_list(guide_dir),
        GuideTemplateAction::Create { name } => template_create(guide_dir, name),
        GuideTemplateAction::Edit { name } => template_edit(guide_dir, name),
        GuideTemplateAction::Delete { name } => template_delete(guide_dir, name),
    }
}

fn template_list(guide_dir: &Path) -> Result<()> {
    let term = Term::stderr();
    let names = guide::list_templates(guide_dir)?;
    if names.is_empty() {
        term.write_line("No guide templates.")?;
        return Ok(());
    }
    for name in &names {
        term.write_line(name)?;
    }
    Ok(())
}

fn template_create(guide_dir: &Path, name: &str) -> Result<()> {
    guide::validate_name(name)?;
    let term = Term::stderr();
    let content = open_editor("")?;
    if content.trim().is_empty() {
        bail!("empty template, nothing saved");
    }
    guide::write_template(guide_dir, name, &content)?;
    term.write_line(&format!(
        "{} Guide template '{name}' created.",
        style("✓").green(),
    ))?;
    Ok(())
}

fn template_edit(guide_dir: &Path, name: &str) -> Result<()> {
    guide::validate_name(name)?;
    let term = Term::stderr();
    let existing = guide::read_template(guide_dir, name)?;
    let content = open_editor(&existing)?;
    if content.trim().is_empty() {
        bail!("empty template, nothing saved");
    }
    guide::write_template(guide_dir, name, &content)?;
    term.write_line(&format!(
        "{} Guide template '{name}' updated.",
        style("✓").green(),
    ))?;
    Ok(())
}

fn template_delete(guide_dir: &Path, name: &str) -> Result<()> {
    guide::validate_name(name)?;
    let term = Term::stderr();
    guide::delete_template(guide_dir, name)?;
    term.write_line(&format!(
        "{} Guide template '{name}' deleted.",
        style("✓").green(),
    ))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Preset subcommands
// ---------------------------------------------------------------------------

fn run_preset(guide_dir: &Path, action: &GuidePresetAction) -> Result<()> {
    match action {
        GuidePresetAction::List { slot } => preset_list(guide_dir, slot.as_deref()),
        GuidePresetAction::Create(args) => preset_create(guide_dir, args),
        GuidePresetAction::Edit { name, slot_preset } => {
            preset_edit(guide_dir, name, slot_preset.as_deref())
        }
        GuidePresetAction::Delete { name, slot_preset } => {
            preset_delete(guide_dir, name, slot_preset.as_deref())
        }
    }
}

fn preset_list(guide_dir: &Path, slot: Option<&str>) -> Result<()> {
    let term = Term::stderr();

    if let Some(s) = slot {
        let names = guide::list_slot_presets(guide_dir, s)?;
        if names.is_empty() {
            term.write_line(&format!("No presets for slot '{s}'."))?;
            return Ok(());
        }
        for name in &names {
            term.write_line(name)?;
        }
        return Ok(());
    }

    let presets = guide::list_presets(guide_dir)?;
    let slots = guide::list_preset_slots(guide_dir)?;

    if presets.is_empty() && slots.is_empty() {
        term.write_line("No presets.")?;
        return Ok(());
    }

    for name in &presets {
        term.write_line(name)?;
    }
    for s in &slots {
        let slot_presets = guide::list_slot_presets(guide_dir, s)?;
        for name in &slot_presets {
            term.write_line(&format!("{s}/{name}"))?;
        }
    }
    Ok(())
}

fn preset_create(guide_dir: &Path, args: &GuidePresetCreateArgs) -> Result<()> {
    guide::validate_name(&args.name)?;
    let term = Term::stderr();

    // Slot preset: ccpick guide preset create <slot> <name>
    if let Some(preset_name) = &args.slot_preset {
        guide::validate_name(preset_name)?;
        let content = open_editor("")?;
        if content.trim().is_empty() {
            bail!("empty preset, nothing saved");
        }
        guide::write_slot_preset(guide_dir, &args.name, preset_name, &content)?;
        term.write_line(&format!(
            "{} Slot preset '{}/{}' created.",
            style("✓").green(),
            args.name,
            preset_name,
        ))?;
        return Ok(());
    }

    // Top-level from template:
    // ccpick guide preset create <name> --from-template <t>
    if let Some(template_name) = &args.from_template {
        let template = guide::read_template(guide_dir, template_name)?;
        let slots = guide::parse_slots(&template)?;
        let fills = pick_slot_presets(guide_dir, &slots)?;
        let rendered = guide::render(&template, &fills)?;
        guide::write_preset(guide_dir, &args.name, &rendered)?;
        term.write_line(&format!(
            "{} Preset '{}' created from template '{}'.",
            style("✓").green(),
            args.name,
            template_name,
        ))?;
        return Ok(());
    }

    // Top-level preset in editor
    let content = open_editor("")?;
    if content.trim().is_empty() {
        bail!("empty preset, nothing saved");
    }
    guide::write_preset(guide_dir, &args.name, &content)?;
    term.write_line(&format!(
        "{} Preset '{}' created.",
        style("✓").green(),
        args.name,
    ))?;
    Ok(())
}

fn preset_edit(guide_dir: &Path, name: &str, slot_preset: Option<&str>) -> Result<()> {
    guide::validate_name(name)?;
    let term = Term::stderr();

    if let Some(preset_name) = slot_preset {
        guide::validate_name(preset_name)?;
        let existing = guide::read_slot_preset(guide_dir, name, preset_name)?;
        let content = open_editor(&existing)?;
        if content.trim().is_empty() {
            bail!("empty preset, nothing saved");
        }
        guide::write_slot_preset(guide_dir, name, preset_name, &content)?;
        term.write_line(&format!(
            "{} Slot preset '{name}/{preset_name}' updated.",
            style("✓").green(),
        ))?;
    } else {
        let existing = guide::read_preset(guide_dir, name)?;
        let content = open_editor(&existing)?;
        if content.trim().is_empty() {
            bail!("empty preset, nothing saved");
        }
        guide::write_preset(guide_dir, name, &content)?;
        term.write_line(&format!("{} Preset '{name}' updated.", style("✓").green(),))?;
    }
    Ok(())
}

fn preset_delete(guide_dir: &Path, name: &str, slot_preset: Option<&str>) -> Result<()> {
    guide::validate_name(name)?;
    let term = Term::stderr();

    if let Some(preset_name) = slot_preset {
        guide::validate_name(preset_name)?;
        guide::delete_slot_preset(guide_dir, name, preset_name)?;
        term.write_line(&format!(
            "{} Slot preset '{name}/{preset_name}' deleted.",
            style("✓").green(),
        ))?;
    } else {
        guide::delete_preset(guide_dir, name)?;
        term.write_line(&format!("{} Preset '{name}' deleted.", style("✓").green(),))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Apply / Compose / Show
// ---------------------------------------------------------------------------

fn apply(guide_dir: &Path, name: Option<&str>) -> Result<()> {
    let term = Term::stderr();
    let name = match name {
        Some(n) => n.to_string(),
        None => pick_from_list("Select a preset", &guide::list_presets(guide_dir)?)?,
    };
    let content = guide::read_preset(guide_dir, &name)?;
    let project_dir = std::env::current_dir()?;
    write_guide_files(&term, &project_dir, &content)?;
    Ok(())
}

fn compose(guide_dir: &Path, name: Option<&str>) -> Result<()> {
    let term = Term::stderr();
    let name = match name {
        Some(n) => n.to_string(),
        None => pick_from_list("Select a template", &guide::list_templates(guide_dir)?)?,
    };
    let template = guide::read_template(guide_dir, &name)?;
    let slots = guide::parse_slots(&template)?;
    let fills = pick_slot_presets(guide_dir, &slots)?;
    let rendered = guide::render(&template, &fills)?;
    let project_dir = std::env::current_dir()?;
    write_guide_files(&term, &project_dir, &rendered)?;
    Ok(())
}

fn show(guide_dir: &Path, name: &str) -> Result<()> {
    // Try as preset first, then template
    let content = if let Ok(c) = guide::read_preset(guide_dir, name) {
        c
    } else {
        guide::read_template(guide_dir, name)?
    };
    let term = Term::stdout();
    term.write_line(&content)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pick_from_list(prompt: &str, items: &[String]) -> Result<String> {
    if items.is_empty() {
        bail!("no items available");
    }
    let selection = Select::new().with_prompt(prompt).items(items).interact()?;
    Ok(items[selection].clone())
}

fn pick_slot_presets(guide_dir: &Path, slots: &[String]) -> Result<BTreeMap<String, String>> {
    let mut fills = BTreeMap::new();
    for slot in slots {
        let presets = guide::list_slot_presets(guide_dir, slot)?;
        if presets.is_empty() {
            bail!(
                "no presets available for slot '{slot}'. \
                 Create one with: ccpick guide preset create {slot} <name>"
            );
        }
        let prompt = format!("Preset for '{slot}'");
        let chosen = pick_from_list(&prompt, &presets)?;
        let content = guide::read_slot_preset(guide_dir, slot, &chosen)?;
        fills.insert(slot.clone(), content);
    }
    Ok(fills)
}

fn write_guide_files(term: &Term, project_dir: &Path, content: &str) -> Result<()> {
    let claude_md = project_dir.join("CLAUDE.md");
    let agents_md = project_dir.join("AGENTS.md");

    for path in [&claude_md, &agents_md] {
        if path.exists() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            let confirmed = Confirm::new()
                .with_prompt(format!("{name} already exists. Overwrite?"))
                .default(false)
                .interact()?;
            if !confirmed {
                bail!("aborted — {name} not overwritten");
            }
        }
    }

    std::fs::write(&claude_md, content)?;
    std::fs::write(&agents_md, content)?;
    term.write_line(&format!(
        "{} Wrote CLAUDE.md and AGENTS.md.",
        style("✓").green(),
    ))?;
    Ok(())
}

fn open_editor(initial: &str) -> Result<String> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let mut tmp = tempfile::Builder::new().suffix(".md").tempfile()?;
    tmp.write_all(initial.as_bytes())?;
    tmp.flush()?;

    let path = tmp.path().to_path_buf();
    let status = std::process::Command::new(&editor).arg(&path).status()?;

    if !status.success() {
        bail!("editor exited with non-zero status");
    }

    std::fs::read_to_string(&path).map_err(Into::into)
}
