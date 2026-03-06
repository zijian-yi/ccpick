use std::path::Path;

use anyhow::{Context, Result};
use console::{Term, style};

use crate::config::Paths;
use crate::manifest::Manifest;
use crate::project;
use crate::symlinks;

pub fn run() -> Result<()> {
    let paths = Paths::resolve()?;
    let term = Term::stderr();
    let project_dir = std::env::current_dir()?;
    run_inner(&term, &paths, &project_dir)
}

fn run_inner(term: &Term, paths: &Paths, project_dir: &Path) -> Result<()> {
    let manifest = Manifest::read(project_dir)?.context(
        "no .claude/ccpick.json found — \
             run `ccpick init` first",
    )?;

    let mut warnings = Vec::new();

    let lib_commands = paths.library_commands();
    let valid_commands: Vec<String> = manifest
        .commands
        .iter()
        .filter(|c| {
            let exists = lib_commands.join(c).exists();
            if !exists {
                warnings.push(format!("command not in library: {c}"));
            }
            exists
        })
        .cloned()
        .collect();

    let lib_skills = paths.library_skills();
    let valid_skills: Vec<String> = manifest
        .skills
        .iter()
        .filter(|s| {
            let exists = lib_skills.join(s).exists();
            if !exists {
                warnings.push(format!("skill not in library: {s}"));
            }
            exists
        })
        .cloned()
        .collect();

    for w in &warnings {
        term.write_line(&format!("  {} {w}", style("⚠").yellow(),))?;
    }

    symlinks::apply(project_dir, paths, &valid_commands, &valid_skills)?;
    let settings_path = project_dir.join(".claude").join("settings.local.json");
    project::merge_enabled_plugins(&settings_path, &manifest.plugins)?;

    term.write_line(&format!(
        "{} Synced: {} command(s), {} skill(s), {} plugin(s)",
        style("✓").green(),
        valid_commands.len(),
        valid_skills.len(),
        manifest.plugins.len(),
    ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use console::Term;
    use serde_json::Value;

    use crate::config::Paths;
    use crate::manifest::Manifest;

    use super::run_inner;

    struct TestEnv {
        _library_tmp: tempfile::TempDir,
        _project_tmp: tempfile::TempDir,
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

        let review = skills.join("review");
        fs::create_dir_all(&review).unwrap();
        fs::write(review.join("SKILL.md"), "# review").unwrap();

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
    fn sync_creates_symlinks_and_settings() {
        let env = setup();

        let mut plugins = BTreeMap::new();
        plugins.insert("my/plugin".to_string(), true);
        plugins.insert("disabled/one".to_string(), false);

        let manifest = Manifest {
            version: 1,
            commands: vec!["foo.md".to_string()],
            skills: vec!["review".to_string()],
            plugins,
        };
        manifest.write(&env.project_dir).unwrap();

        let term = Term::stderr();
        run_inner(&term, &env.paths, &env.project_dir).unwrap();

        let cmd_link = env.project_dir.join(".claude/commands/foo.md");
        assert!(cmd_link.symlink_metadata().unwrap().is_symlink());

        let skill_link = env.project_dir.join(".claude/skills/review");
        assert!(skill_link.symlink_metadata().unwrap().is_symlink());
        assert!(
            skill_link.join("SKILL.md").exists(),
            "skill dir contents accessible through symlink",
        );

        let settings_path = env.project_dir.join(".claude/settings.local.json");
        let contents = fs::read_to_string(&settings_path).unwrap();
        let val: Value = serde_json::from_str(&contents).unwrap();
        let ep = val.get("enabledPlugins").unwrap();
        assert_eq!(ep.get("my/plugin").unwrap(), true);
        assert_eq!(ep.get("disabled/one").unwrap(), false);
    }

    #[test]
    fn sync_skips_missing_items_with_warnings() {
        let env = setup();

        let manifest = Manifest {
            version: 1,
            commands: vec!["foo.md".to_string(), "ghost.md".to_string()],
            skills: vec!["missing-skill".to_string()],
            plugins: BTreeMap::new(),
        };
        manifest.write(&env.project_dir).unwrap();

        let term = Term::stderr();
        run_inner(&term, &env.paths, &env.project_dir).unwrap();

        let cmd_link = env.project_dir.join(".claude/commands/foo.md");
        assert!(cmd_link.symlink_metadata().is_ok());

        let ghost = env.project_dir.join(".claude/commands/ghost.md");
        assert!(ghost.symlink_metadata().is_err());
    }

    #[test]
    fn sync_errors_without_manifest() {
        let env = setup();
        let term = Term::stderr();
        let result = run_inner(&term, &env.paths, &env.project_dir);
        assert!(result.is_err());
    }
}
