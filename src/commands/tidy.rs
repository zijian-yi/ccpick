use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use console::{Term, style};
use dialoguer::MultiSelect;

use crate::cli::TidyArgs;
use crate::commands::init;
use crate::config::Paths;
use crate::plugins;
use crate::project;
use crate::scanner;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Location {
    Global,
    Library,
}

struct Item {
    rel_path: PathBuf,
    category: &'static str,
    location: Location,
}

impl Item {
    fn label(&self) -> String {
        format!("[{}] {}", self.category, self.rel_path.display())
    }
}

fn collect_items(dir: &Path, category: &'static str, location: Location) -> Result<Vec<Item>> {
    let paths = if category == "skills" {
        scanner::scan_skill_dirs(dir)?
    } else {
        scanner::scan_md_files(dir)?
    };
    Ok(paths
        .into_iter()
        .map(|rel_path| Item {
            rel_path,
            category,
            location,
        })
        .collect())
}

fn move_item(src: &Path, dst: &Path, src_root: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    fs::rename(src, dst)
        .with_context(|| format!("moving {} → {}", src.display(), dst.display()))?;

    let mut dir = src.parent();
    while let Some(d) = dir {
        if d == src_root {
            break;
        }
        if fs::read_dir(d).is_ok_and(|mut e| e.next().is_none()) {
            let _ = fs::remove_dir(d);
        } else {
            break;
        }
        dir = d.parent();
    }

    Ok(())
}

fn resolve_dirs(item: &Item, paths: &Paths) -> Result<(PathBuf, PathBuf)> {
    match item.category {
        "commands" => Ok((paths.global_commands.clone(), paths.library_commands())),
        "skills" => Ok((paths.global_skills.clone(), paths.library_skills())),
        _ => anyhow::bail!("unknown category: {}", item.category),
    }
}

pub fn run(args: &TidyArgs) -> Result<()> {
    let paths = Paths::resolve()?;
    let term = Term::stderr();

    let categories: Vec<&str> = [
        args.want_commands().then_some("commands"),
        args.want_skills().then_some("skills"),
    ]
    .into_iter()
    .flatten()
    .collect();

    if !categories.is_empty() {
        tidy_commands_skills(&term, &paths, &categories)?;
    }
    if args.want_plugins() {
        tidy_plugins(&term, &paths)?;
    }

    Ok(())
}

fn tidy_commands_skills(term: &Term, paths: &Paths, categories: &[&'static str]) -> Result<()> {
    let mut items = Vec::new();
    for &category in categories {
        let (global_dir, library_dir) = match category {
            "commands" => (&paths.global_commands, paths.library_commands()),
            _ => (&paths.global_skills, paths.library_skills()),
        };
        items.extend(collect_items(global_dir, category, Location::Global)?);
        items.extend(collect_items(&library_dir, category, Location::Library)?);
    }

    if items.is_empty() {
        term.write_line(&format!(
            "{} No commands or skills found.",
            style("✓").green(),
        ))?;
        return Ok(());
    }

    items.sort_by(|a, b| a.category.cmp(b.category).then(a.rel_path.cmp(&b.rel_path)));

    let labels: Vec<String> = items.iter().map(Item::label).collect();
    let defaults: Vec<bool> = items
        .iter()
        .map(|item| item.location == Location::Library)
        .collect();

    term.write_line(
        "Checked = managed by ccpick, \
         unchecked = stays global.\n\
         Space to toggle, Enter to confirm:",
    )?;

    let Some(selected) = MultiSelect::new()
        .items(&labels)
        .defaults(&defaults)
        .interact_opt()?
    else {
        return Err(crate::UserAbort.into());
    };

    let mut moved_count = 0u32;

    for (idx, item) in items.iter().enumerate() {
        let want_library = selected.contains(&idx);
        if want_library == (item.location == Location::Library) {
            continue;
        }

        let (global_dir, library_dir) = resolve_dirs(item, paths)?;

        if want_library {
            let src = global_dir.join(&item.rel_path);
            let dst = library_dir.join(&item.rel_path);
            move_item(&src, &dst, &global_dir)?;
            term.write_line(&format!(
                "  {} {} → ccpick",
                style("→").cyan(),
                item.label(),
            ))?;
        } else {
            let src = library_dir.join(&item.rel_path);
            let dst = global_dir.join(&item.rel_path);
            move_item(&src, &dst, &library_dir)?;
            term.write_line(&format!(
                "  {} {} → global",
                style("←").yellow(),
                item.label(),
            ))?;
        }

        moved_count += 1;
    }

    if moved_count == 0 {
        term.write_line("No changes to commands/skills.")?;
    } else {
        term.write_line(&format!(
            "\n{} Moved {moved_count} item(s).",
            style("✓").green(),
        ))?;
    }

    Ok(())
}

fn tidy_plugins(term: &Term, paths: &Paths) -> Result<()> {
    let plugin_infos = plugins::scan_plugins(&paths.claude_home)?;

    if plugin_infos.is_empty() {
        term.write_line("\n  No plugins installed, skipping.")?;
        return Ok(());
    }

    let current = project::read_global_enabled_plugins(paths)?;
    let plugin_map = init::pick_plugins(
        term,
        &plugin_infos,
        Some(&current),
        "Select plugins to enable globally",
    )?;

    let settings_path = paths.claude_home.join("settings.json");
    project::merge_enabled_plugins(&settings_path, &plugin_map)?;

    let enabled = plugin_map.values().filter(|v| **v).count();
    term.write_line(&format!(
        "{} Updated global plugin defaults: \
         {enabled}/{} enabled",
        style("✓").green(),
        plugin_map.len(),
    ))?;

    Ok(())
}
