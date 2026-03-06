use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Paths;

/// Create symlinks in the project's `.claude/{commands,skills}/` pointing
/// to items in the ccpick library. Removes stale symlinks that point into
/// the library but aren't in the selection.
pub fn apply(
    project_dir: &Path,
    paths: &Paths,
    commands: &[String],
    skills: &[String],
) -> Result<()> {
    apply_category(project_dir, "commands", &paths.library_commands(), commands)?;
    apply_category(project_dir, "skills", &paths.library_skills(), skills)?;
    update_gitignore(project_dir, commands, skills)?;
    Ok(())
}

const MARKER_START: &str = "# ccpick managed (do not edit this block)";
const MARKER_END: &str = "# end ccpick managed";

fn update_gitignore(project_dir: &Path, commands: &[String], skills: &[String]) -> Result<()> {
    let path = project_dir.join(".claude").join(".gitignore");

    let existing = if path.exists() {
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };

    let outside = strip_managed_block(&existing);

    let mut block = String::new();
    block.push_str(MARKER_START);
    block.push('\n');
    for cmd in commands {
        let _ = writeln!(block, "commands/{cmd}");
    }
    for skill in skills {
        let _ = writeln!(block, "skills/{skill}");
    }
    block.push_str(MARKER_END);
    block.push('\n');

    let mut result = String::new();
    if !outside.is_empty() {
        result.push_str(&outside);
        if !outside.ends_with('\n') {
            result.push('\n');
        }
    }
    result.push_str(&block);

    fs::write(&path, &result).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn strip_managed_block(content: &str) -> String {
    let mut result = String::new();
    let mut in_block = false;
    for line in content.lines() {
        if line == MARKER_START {
            in_block = true;
            continue;
        }
        if line == MARKER_END {
            in_block = false;
            continue;
        }
        if !in_block {
            result.push_str(line);
            result.push('\n');
        }
    }
    // Trim trailing newlines to avoid accumulation
    let trimmed = result.trim_end_matches('\n');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

fn apply_category(
    project_dir: &Path,
    category: &str,
    library_dir: &Path,
    selected: &[String],
) -> Result<()> {
    let target_dir = project_dir.join(".claude").join(category);
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating {}", target_dir.display()))?;

    remove_stale_symlinks(&target_dir, library_dir)?;

    for rel in selected {
        let link_path = target_dir.join(rel);
        let source = library_dir.join(rel);

        if !source.exists() {
            continue;
        }

        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            continue;
        }

        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }

        symlink(&source, &link_path).with_context(|| {
            format!("symlinking {} → {}", link_path.display(), source.display(),)
        })?;
    }

    Ok(())
}

/// Remove symlinks inside `target_dir` that point into `library_dir`.
/// Leaves non-symlinks and symlinks pointing elsewhere untouched.
fn remove_stale_symlinks(target_dir: &Path, library_dir: &Path) -> Result<()> {
    if !target_dir.is_dir() {
        return Ok(());
    }
    remove_stale_recursive(target_dir, library_dir)
}

fn remove_stale_recursive(dir: &Path, library_dir: &Path) -> Result<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let Ok(meta) = path.symlink_metadata() else {
            continue;
        };

        if meta.is_dir() {
            remove_stale_recursive(&path, library_dir)?;
            if fs::read_dir(&path).is_ok_and(|mut e| e.next().is_none()) {
                let _ = fs::remove_dir(&path);
            }
        } else if meta.is_symlink()
            && let Ok(target) = fs::read_link(&path)
            && target.starts_with(library_dir)
        {
            fs::remove_file(&path)
                .with_context(|| format!("removing stale symlink {}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    use super::*;

    struct TestEnv {
        _project_tmp: tempfile::TempDir,
        _library_tmp: tempfile::TempDir,
        project_dir: PathBuf,
        paths: Paths,
    }

    fn setup() -> TestEnv {
        let library_tmp = tempfile::tempdir().unwrap();
        let project_tmp = tempfile::tempdir().unwrap();

        let lib = library_tmp.path();
        let commands = lib.join("commands");
        let skills = lib.join("skills");
        fs::create_dir_all(&commands).unwrap();
        fs::create_dir_all(&skills).unwrap();

        fs::write(commands.join("foo.md"), "# foo").unwrap();
        fs::create_dir_all(commands.join("sub")).unwrap();
        fs::write(commands.join("sub/bar.md"), "# bar").unwrap();

        let review = skills.join("review");
        fs::create_dir_all(&review).unwrap();
        fs::write(review.join("SKILL.md"), "# review").unwrap();
        fs::write(review.join("data.txt"), "extra").unwrap();

        let paths = Paths {
            claude_home: lib.to_path_buf(),
            library: lib.to_path_buf(),
            global_commands: lib.join("global_commands"),
            global_skills: lib.join("global_skills"),
        };

        TestEnv {
            project_dir: project_tmp.path().to_path_buf(),
            _project_tmp: project_tmp,
            _library_tmp: library_tmp,
            paths,
        }
    }

    #[test]
    fn creates_symlinks_for_selected_items() {
        let env = setup();
        apply(
            &env.project_dir,
            &env.paths,
            &["foo.md".to_string(), "sub/bar.md".to_string()],
            &["review".to_string()],
        )
        .unwrap();

        let cmd_link = env.project_dir.join(".claude/commands/foo.md");
        assert!(cmd_link.symlink_metadata().unwrap().is_symlink());
        assert_eq!(
            fs::read_link(&cmd_link).unwrap(),
            env.paths.library_commands().join("foo.md"),
        );

        let nested = env.project_dir.join(".claude/commands/sub/bar.md");
        assert!(nested.symlink_metadata().unwrap().is_symlink());

        let skill_link = env.project_dir.join(".claude/skills/review");
        assert!(
            skill_link.symlink_metadata().unwrap().is_symlink(),
            "skill directory should be symlinked",
        );
        assert!(
            skill_link.join("SKILL.md").exists(),
            "files inside skill dir accessible through symlink",
        );
    }

    #[test]
    fn skips_missing_library_items() {
        let env = setup();
        apply(
            &env.project_dir,
            &env.paths,
            &["nonexistent.md".to_string()],
            &[],
        )
        .unwrap();

        let link = env.project_dir.join(".claude/commands/nonexistent.md");
        assert!(!link.exists());
        assert!(link.symlink_metadata().is_err());
    }

    #[test]
    fn removes_stale_library_symlinks() {
        let env = setup();

        let cmd_dir = env.project_dir.join(".claude").join("commands");
        fs::create_dir_all(&cmd_dir).unwrap();

        let stale = cmd_dir.join("stale.md");
        symlink(env.paths.library_commands().join("stale.md"), &stale).unwrap();

        apply(&env.project_dir, &env.paths, &["foo.md".to_string()], &[]).unwrap();

        assert!(
            stale.symlink_metadata().is_err(),
            "stale symlink should be removed"
        );
        assert!(cmd_dir.join("foo.md").symlink_metadata().is_ok());
    }

    #[test]
    fn preserves_foreign_files_and_symlinks() {
        let env = setup();

        let cmd_dir = env.project_dir.join(".claude").join("commands");
        fs::create_dir_all(&cmd_dir).unwrap();

        let real_file = cmd_dir.join("custom.md");
        fs::write(&real_file, "# custom").unwrap();

        let foreign_target = tempfile::tempdir().unwrap();
        let foreign_file = foreign_target.path().join("ext.md");
        fs::write(&foreign_file, "# ext").unwrap();
        let foreign_link = cmd_dir.join("foreign.md");
        symlink(&foreign_file, &foreign_link).unwrap();

        apply(&env.project_dir, &env.paths, &["foo.md".to_string()], &[]).unwrap();

        assert!(real_file.exists(), "real file preserved");
        assert!(
            foreign_link.symlink_metadata().is_ok(),
            "foreign symlink preserved"
        );
    }

    #[test]
    fn removes_stale_skill_dir_symlinks() {
        let env = setup();

        let skill_dir = env.project_dir.join(".claude").join("skills");
        fs::create_dir_all(&skill_dir).unwrap();

        let stale = skill_dir.join("old-skill");
        symlink(env.paths.library_skills().join("old-skill"), &stale).unwrap();

        apply(&env.project_dir, &env.paths, &[], &["review".to_string()]).unwrap();

        assert!(
            stale.symlink_metadata().is_err(),
            "stale skill dir symlink should be removed"
        );
        assert!(skill_dir.join("review").symlink_metadata().is_ok(),);
    }

    #[test]
    fn cleans_empty_subdirs_after_stale_removal() {
        let env = setup();

        let sub_dir = env.project_dir.join(".claude").join("commands").join("sub");
        fs::create_dir_all(&sub_dir).unwrap();

        let stale = sub_dir.join("old.md");
        symlink(env.paths.library_commands().join("sub/old.md"), &stale).unwrap();

        apply(&env.project_dir, &env.paths, &[], &[]).unwrap();

        assert!(!sub_dir.exists(), "empty subdir should be removed");
    }

    #[test]
    fn gitignore_lists_specific_entries() {
        let env = setup();
        apply(
            &env.project_dir,
            &env.paths,
            &["foo.md".to_string()],
            &["review".to_string()],
        )
        .unwrap();

        let gi = fs::read_to_string(env.project_dir.join(".claude/.gitignore")).unwrap();
        assert!(gi.contains("commands/foo.md\n"));
        assert!(gi.contains("skills/review\n"));
        assert!(!gi.contains("commands/\n"));
    }

    #[test]
    fn gitignore_preserves_user_entries() {
        let env = setup();
        let claude_dir = env.project_dir.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join(".gitignore"), "my-custom-entry\n").unwrap();

        apply(&env.project_dir, &env.paths, &["foo.md".to_string()], &[]).unwrap();

        let gi = fs::read_to_string(claude_dir.join(".gitignore")).unwrap();
        assert!(gi.contains("my-custom-entry\n"));
        assert!(gi.contains("commands/foo.md\n"));
    }

    #[test]
    fn gitignore_updates_managed_block() {
        let env = setup();

        apply(
            &env.project_dir,
            &env.paths,
            &["foo.md".to_string()],
            &["review".to_string()],
        )
        .unwrap();

        // Re-apply with different selections
        apply(
            &env.project_dir,
            &env.paths,
            &["sub/bar.md".to_string()],
            &[],
        )
        .unwrap();

        let gi = fs::read_to_string(env.project_dir.join(".claude/.gitignore")).unwrap();
        assert!(gi.contains("commands/sub/bar.md\n"));
        assert!(!gi.contains("commands/foo.md"));
        assert!(!gi.contains("skills/review"));
    }

    #[test]
    fn gitignore_empty_selections() {
        let env = setup();
        apply(&env.project_dir, &env.paths, &[], &[]).unwrap();

        let gi = fs::read_to_string(env.project_dir.join(".claude/.gitignore")).unwrap();
        assert!(gi.contains(MARKER_START));
        assert!(gi.contains(MARKER_END));
    }
}
