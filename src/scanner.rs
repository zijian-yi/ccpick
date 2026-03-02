use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Recursively scan a directory for `.md` files, returning paths
/// relative to `dir`.
pub fn scan_md_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut items = Vec::new();
    if !dir.is_dir() {
        return Ok(items);
    }
    collect_md_recursive(dir, dir, &mut items)?;
    items.sort();
    Ok(items)
}

/// Scan for skill directories -- folders containing a `skill.md`
/// (case-insensitive). Returns relative paths of those folders.
/// Does not recurse into a matched skill folder.
pub fn scan_skill_dirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut items = Vec::new();
    if !dir.is_dir() {
        return Ok(items);
    }
    collect_skill_dirs_recursive(dir, dir, &mut items)?;
    items.sort();
    Ok(items)
}

fn collect_md_recursive(
    root: &Path,
    dir: &Path,
    items: &mut Vec<PathBuf>,
) -> Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_md_recursive(root, &path, items)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            let rel = path.strip_prefix(root).with_context(|| {
                format!(
                    "stripping prefix {} from {}",
                    root.display(),
                    path.display()
                )
            })?;
            items.push(rel.to_path_buf());
        }
    }
    Ok(())
}

fn collect_skill_dirs_recursive(
    root: &Path,
    dir: &Path,
    items: &mut Vec<PathBuf>,
) -> Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if contains_skill_md(&path)? {
            let rel =
                path.strip_prefix(root).with_context(|| {
                    format!(
                        "stripping prefix {} from {}",
                        root.display(),
                        path.display()
                    )
                })?;
            items.push(rel.to_path_buf());
        } else {
            collect_skill_dirs_recursive(root, &path, items)?;
        }
    }
    Ok(())
}

pub fn contains_skill_md(dir: &Path) -> Result<bool> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str()
            && name.eq_ignore_ascii_case("skill.md")
            && entry.path().is_file()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn finds_md_files_recursively() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::write(root.join("foo.md"), "# foo").unwrap();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/bar.md"), "# bar").unwrap();
        fs::write(root.join("sub/ignored.txt"), "nope").unwrap();

        let results = scan_md_files(root).unwrap();
        assert_eq!(
            results,
            vec![
                PathBuf::from("foo.md"),
                PathBuf::from("sub/bar.md"),
            ],
        );
    }

    #[test]
    fn md_missing_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let results =
            scan_md_files(&tmp.path().join("nonexistent")).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn md_empty_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let results = scan_md_files(tmp.path()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn finds_skill_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let skill_a = root.join("my-skill");
        fs::create_dir_all(&skill_a).unwrap();
        fs::write(skill_a.join("SKILL.md"), "# skill").unwrap();
        fs::write(skill_a.join("helper.py"), "pass").unwrap();

        let skill_b = root.join("another");
        fs::create_dir_all(&skill_b).unwrap();
        fs::write(skill_b.join("skill.md"), "# skill").unwrap();

        let results = scan_skill_dirs(root).unwrap();
        assert_eq!(
            results,
            vec![
                PathBuf::from("another"),
                PathBuf::from("my-skill"),
            ],
        );
    }

    #[test]
    fn skill_case_insensitive() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let skill = root.join("mixed");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("Skill.MD"), "# skill").unwrap();

        let results = scan_skill_dirs(root).unwrap();
        assert_eq!(results, vec![PathBuf::from("mixed")]);
    }

    #[test]
    fn skill_nested_under_category() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let skill = root.join("category").join("deep-skill");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "# s").unwrap();

        let results = scan_skill_dirs(root).unwrap();
        assert_eq!(
            results,
            vec![PathBuf::from("category/deep-skill")],
        );
    }

    #[test]
    fn skill_does_not_recurse_into_matched_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let outer = root.join("outer");
        fs::create_dir_all(&outer).unwrap();
        fs::write(outer.join("SKILL.md"), "# outer").unwrap();

        let inner = outer.join("inner");
        fs::create_dir_all(&inner).unwrap();
        fs::write(inner.join("SKILL.md"), "# inner").unwrap();

        let results = scan_skill_dirs(root).unwrap();
        assert_eq!(
            results,
            vec![PathBuf::from("outer")],
            "should not find nested skill inside a skill",
        );
    }

    #[test]
    fn skill_ignores_dirs_without_skill_md() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let no_skill = root.join("just-a-dir");
        fs::create_dir_all(&no_skill).unwrap();
        fs::write(no_skill.join("readme.md"), "# hi").unwrap();

        let results = scan_skill_dirs(root).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn skill_missing_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let results = scan_skill_dirs(
            &tmp.path().join("nonexistent"),
        )
        .unwrap();
        assert!(results.is_empty());
    }
}
