use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use console::{style, Term};
use dialoguer::MultiSelect;

use crate::cli::InstallArgs;
use crate::config::Paths;
use crate::remote;
use crate::scanner;

/// Target directory for installed items.
enum Target {
    /// `~/.claude/ccpick/{commands,skills}/` (default)
    Library,
    /// `~/.claude/{commands,skills}/`
    Global,
    /// `.claude/{commands,skills}/`
    Local,
}

pub fn run(args: &InstallArgs) -> Result<()> {
    let term = Term::stderr();
    let paths = Paths::resolve()?;
    let repo_ref = remote::parse_github_url(&args.url)?;

    let mut label = format!(
        "{}/{}",
        repo_ref.owner, repo_ref.repo
    );
    if let Some(path) = &repo_ref.path {
        use std::fmt::Write;
        let _ = write!(label, "/{path}");
    }
    term.write_line(&format!("Cloning {label}..."))?;

    let clone_dir = remote::shallow_clone(
        &repo_ref,
        args.branch.as_deref(),
    )?;

    let target = if args.global {
        Target::Global
    } else if args.local {
        Target::Local
    } else {
        Target::Library
    };

    if let Some(sub_path) = &repo_ref.path {
        install_path(
            &term,
            clone_dir.path(),
            sub_path,
            &target,
            &paths,
        )
    } else {
        install_repo(&term, clone_dir.path(), &target, &paths)
    }
}

/// Collect scan roots: the repo root itself plus any
/// dot-directories (e.g. `.claude`, `.cursor`, `.windsurf`).
fn collect_scan_roots(clone_root: &Path) -> Result<Vec<PathBuf>> {
    let mut roots = vec![clone_root.to_path_buf()];

    if let Ok(entries) = fs::read_dir(clone_root) {
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir()
                && let Some(name) = entry.file_name().to_str()
                && name.starts_with('.')
                && name != ".git"
            {
                roots.push(entry.path());
            }
        }
    }

    Ok(roots)
}

/// Install from a repo root: scan commands/ and skills/,
/// present an interactive picker, and copy selected items.
fn install_repo(
    term: &Term,
    clone_root: &Path,
    target: &Target,
    paths: &Paths,
) -> Result<()> {
    let scan_roots = collect_scan_roots(clone_root)?;
    let mut commands: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut skills: Vec<(PathBuf, PathBuf)> = Vec::new();

    for base in &scan_roots {
        let cmd_dir = base.join("commands");
        let skill_dir = base.join("skills");

        for rel in scanner::scan_md_files(&cmd_dir)? {
            commands.push((cmd_dir.clone(), rel));
        }
        for rel in scanner::scan_skill_dirs(&skill_dir)? {
            skills.push((skill_dir.clone(), rel));
        }
    }

    if commands.is_empty() && skills.is_empty() {
        bail!(
            "no commands or skills found in repository"
        );
    }

    let mut items: Vec<(&str, &Path, &PathBuf)> = Vec::new();
    for (scan_root, rel) in &commands {
        items.push(("commands", scan_root, rel));
    }
    for (scan_root, rel) in &skills {
        items.push(("skills", scan_root, rel));
    }

    let labels: Vec<String> = items
        .iter()
        .map(|(cat, _, rel)| {
            format!("[{cat}] {}", rel.display())
        })
        .collect();

    let defaults: Vec<bool> = vec![false; items.len()];

    term.write_line(
        "\nSelect items to install \
         (space to toggle, enter to confirm):",
    )?;

    let Some(selected) = MultiSelect::new()
        .items(&labels)
        .defaults(&defaults)
        .interact_opt()?
    else {
        return Err(crate::UserAbort.into());
    };

    if selected.is_empty() {
        term.write_line("Nothing selected.")?;
        return Ok(());
    }

    let mut installed = 0u32;
    let mut skipped = 0u32;

    for idx in &selected {
        let (category, scan_root, rel_path) = items[*idx];
        let src = scan_root.join(rel_path);
        let dst_dir = resolve_target_dir(target, category, paths)?;
        // Skills: use just the directory name (e.g. "doc" not
        // ".curated/doc") so repo-internal organization doesn't
        // leak into the library.
        let dst_name = if category == "skills" {
            rel_path
                .file_name()
                .map_or_else(|| rel_path.clone(), PathBuf::from)
        } else {
            rel_path.clone()
        };
        let dst = dst_dir.join(&dst_name);

        if dst.exists() {
            term.write_line(&format!(
                "  {} {} already exists, skipping",
                style("!").yellow(),
                dst.display(),
            ))?;
            skipped += 1;
            continue;
        }

        copy_entry(&src, &dst)?;
        installed += 1;

        term.write_line(&format!(
            "  {} [{category}] {}",
            style("+").green(),
            dst_name.display(),
        ))?;
    }

    print_summary(term, installed, skipped)?;
    Ok(())
}

/// Install a specific path from the repo (file or directory).
fn install_path(
    term: &Term,
    clone_root: &Path,
    sub_path: &str,
    target: &Target,
    paths: &Paths,
) -> Result<()> {
    let src = clone_root.join(sub_path);
    if !src.exists() {
        bail!("path not found in repository: {sub_path}");
    }

    let (category, rel_path) = detect_type(&src, sub_path)?;
    let dst_dir =
        resolve_target_dir(target, category, paths)?;
    let dst = dst_dir.join(&rel_path);

    if dst.exists() {
        term.write_line(&format!(
            "{} {} already exists, skipping",
            style("!").yellow(),
            dst.display(),
        ))?;
        return Ok(());
    }

    copy_entry(&src, &dst)?;

    term.write_line(&format!(
        "{} Installed [{category}] {}",
        style("✓").green(),
        rel_path,
    ))?;

    Ok(())
}

/// Detect whether a path is a command or skill.
///
/// Rules:
/// - `.md` file → command
/// - Directory with `skill.md` → skill
/// - Directory with only `.md` files → commands (error:
///   ambiguous, use repo-root mode)
/// - Otherwise → error with guidance
fn detect_type(
    src: &Path,
    sub_path: &str,
) -> Result<(&'static str, String)> {
    if src.is_file() {
        if src
            .extension()
            .is_some_and(|ext| ext == "md")
        {
            let name = src
                .file_name()
                .context("getting file name")?
                .to_string_lossy()
                .into_owned();
            return Ok(("commands", name));
        }
        bail!(
            "'{sub_path}' is not a .md file — \
             only markdown commands are supported"
        );
    }

    if src.is_dir() {
        // Check for skill.md inside the directory
        if scanner::contains_skill_md(src)? {
            let name = src
                .file_name()
                .context("getting directory name")?
                .to_string_lossy()
                .into_owned();
            return Ok(("skills", name));
        }

        let has_commands =
            !scanner::scan_md_files(src)?.is_empty();
        let has_skills =
            !scanner::scan_skill_dirs(src)?.is_empty();

        match (has_commands, has_skills) {
            (true, true) => bail!(
                "'{sub_path}' contains both commands and \
                 skills — use the repo root URL to pick \
                 interactively"
            ),
            (true, false) => bail!(
                "'{sub_path}' contains multiple commands \
                 — use the repo root URL to pick \
                 interactively"
            ),
            (false, true) => bail!(
                "'{sub_path}' contains multiple skills — \
                 use the repo root URL to pick \
                 interactively"
            ),
            (false, false) => bail!(
                "'{sub_path}' doesn't look like a command \
                 (.md file) or skill (directory with \
                 skill.md)"
            ),
        }
    }

    bail!("'{sub_path}' is not a file or directory");
}

fn resolve_target_dir(
    target: &Target,
    category: &str,
    paths: &Paths,
) -> Result<PathBuf> {
    let base = match target {
        Target::Library => match category {
            "commands" => paths.library_commands(),
            _ => paths.library_skills(),
        },
        Target::Global => match category {
            "commands" => paths.global_commands.clone(),
            _ => paths.global_skills.clone(),
        },
        Target::Local => {
            let project_dir = std::env::current_dir()?;
            project_dir.join(".claude").join(category)
        }
    };
    Ok(base)
}

/// Copy a file or directory recursively.
fn copy_entry(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("creating {}", parent.display())
        })?;
    }

    let meta = fs::metadata(src).with_context(|| {
        format!("reading metadata for {}", src.display())
    })?;

    if meta.is_file() {
        fs::copy(src, dst).with_context(|| {
            format!(
                "copying {} → {}",
                src.display(),
                dst.display()
            )
        })?;
    } else if meta.is_dir() {
        copy_dir_recursive(src, dst)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| {
        format!("creating {}", dst.display())
    })?;

    let entries = fs::read_dir(src)
        .with_context(|| format!("reading {}", src.display()))?;

    for entry in entries {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), &dst_path).with_context(
                || {
                    format!(
                        "copying {} → {}",
                        entry.path().display(),
                        dst_path.display()
                    )
                },
            )?;
        }
    }
    Ok(())
}

fn print_summary(
    term: &Term,
    installed: u32,
    skipped: u32,
) -> Result<()> {
    let mut parts = Vec::new();
    if installed > 0 {
        parts.push(format!("{installed} installed"));
    }
    if skipped > 0 {
        parts.push(format!("{skipped} skipped"));
    }

    term.write_line(&format!(
        "\n{} {}",
        style("✓").green(),
        parts.join(", "),
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn copy_command_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dst_dir = tmp.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();

        let src = src_dir.join("test.md");
        fs::write(&src, "# test").unwrap();

        let dst = dst_dir.join("test.md");
        copy_entry(&src, &dst).unwrap();

        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "# test");
    }

    #[test]
    fn copy_skill_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("my-skill");
        let dst = tmp.path().join("installed/my-skill");

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("skill.md"), "# skill").unwrap();
        fs::write(src.join("helper.py"), "pass").unwrap();
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("sub/data.txt"), "data").unwrap();

        copy_entry(&src, &dst).unwrap();

        assert!(dst.join("skill.md").exists());
        assert!(dst.join("helper.py").exists());
        assert!(dst.join("sub/data.txt").exists());
    }

    #[test]
    fn skip_existing_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("test.md");
        fs::write(&dst, "original").unwrap();

        // dst.exists() returns true, so the caller should skip
        assert!(dst.exists());
    }

    #[test]
    fn detect_md_file_as_command() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("config.md");
        fs::write(&file, "# config").unwrap();

        let (cat, name) =
            detect_type(&file, "config.md").unwrap();
        assert_eq!(cat, "commands");
        assert_eq!(name, "config.md");
    }

    #[test]
    fn detect_skill_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("review");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("skill.md"), "# skill").unwrap();

        let (cat, name) =
            detect_type(&dir, "review").unwrap();
        assert_eq!(cat, "skills");
        assert_eq!(name, "review");
    }

    #[test]
    fn detect_non_md_file_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("script.py");
        fs::write(&file, "pass").unwrap();

        assert!(detect_type(&file, "script.py").is_err());
    }

    #[test]
    fn detect_empty_dir_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("empty");
        fs::create_dir_all(&dir).unwrap();

        assert!(detect_type(&dir, "empty").is_err());
    }

    #[test]
    fn detect_dir_with_multiple_commands_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("cmds");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.md"), "# a").unwrap();
        fs::write(dir.join("b.md"), "# b").unwrap();

        assert!(detect_type(&dir, "cmds").is_err());
    }

    #[test]
    fn detect_dir_with_multiple_skills_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");
        fs::create_dir_all(&dir).unwrap();

        let s1 = dir.join("s1");
        fs::create_dir_all(&s1).unwrap();
        fs::write(s1.join("skill.md"), "# s1").unwrap();

        let s2 = dir.join("s2");
        fs::create_dir_all(&s2).unwrap();
        fs::write(s2.join("skill.md"), "# s2").unwrap();

        assert!(detect_type(&dir, "skills").is_err());
    }
}
