use std::process::Command;

use anyhow::{Context, Result, bail};

pub struct RepoRef {
    pub owner: String,
    pub repo: String,
    /// Branch parsed from URL (e.g. from `/tree/main/...`)
    pub branch: Option<String>,
    /// Sub-path within repo (e.g. `commands/foo.md`)
    pub path: Option<String>,
}

impl RepoRef {
    pub fn clone_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.repo)
    }
}

/// Parse a GitHub URL into its components.
///
/// Accepted patterns:
/// - `https://github.com/owner/repo`
/// - `https://github.com/owner/repo/tree/branch/path`
/// - `https://github.com/owner/repo/blob/branch/path`
/// - `github.com/owner/repo` (no scheme)
/// - `owner/repo` (shorthand)
/// - Trailing `.git` is stripped
pub fn parse_github_url(url: &str) -> Result<RepoRef> {
    let url = url.trim();
    if url.is_empty() {
        bail!("empty URL");
    }

    // Strip scheme if present
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Split into host part and path
    let (host, path_str) = if let Some(rest) = without_scheme.strip_prefix("github.com/") {
        ("github.com", rest)
    } else if without_scheme.contains("github.com") {
        bail!("unrecognized GitHub URL format: {url}");
    } else {
        // Treat as owner/repo shorthand
        ("github.com", without_scheme)
    };

    if host != "github.com" {
        bail!("only GitHub URLs are supported: {url}");
    }

    let path_str = path_str.trim_end_matches('/');
    if path_str.is_empty() {
        bail!("missing owner/repo in URL: {url}");
    }

    let segments: Vec<&str> = path_str.split('/').collect();
    if segments.len() < 2 {
        bail!("missing repo name in URL: {url}");
    }

    let owner = segments[0].to_string();
    let repo = segments[1]
        .strip_suffix(".git")
        .unwrap_or(segments[1])
        .to_string();

    if owner.is_empty() || repo.is_empty() {
        bail!("missing owner or repo in URL: {url}");
    }

    // Parse optional tree/blob path: /tree/branch/path or
    // /blob/branch/path
    let (branch, sub_path) = if segments.len() > 2 {
        let kind = segments[2];
        if kind != "tree" && kind != "blob" {
            bail!(
                "unexpected URL segment '{kind}' \
                 (expected 'tree' or 'blob'): {url}"
            );
        }
        if segments.len() < 4 {
            bail!("missing branch in URL: {url}");
        }
        let branch = segments[3].to_string();
        let path = if segments.len() > 4 {
            Some(segments[4..].join("/"))
        } else {
            None
        };
        (Some(branch), path)
    } else {
        (None, None)
    };

    Ok(RepoRef {
        owner,
        repo,
        branch,
        path: sub_path,
    })
}

/// Shallow-clone a GitHub repo into a temporary directory.
///
/// Uses `git clone --depth 1` for speed. The `branch_override`
/// takes precedence over the branch parsed from the URL.
pub fn shallow_clone(
    repo_ref: &RepoRef,
    branch_override: Option<&str>,
) -> Result<tempfile::TempDir> {
    let tmp = tempfile::tempdir().context("creating temporary directory for clone")?;

    let branch = branch_override.or(repo_ref.branch.as_deref());

    let mut cmd = Command::new("git");
    cmd.args(["clone", "--depth", "1"]);
    if let Some(b) = branch {
        cmd.args(["--branch", b]);
    }
    cmd.arg(repo_ref.clone_url());
    cmd.arg(tmp.path());

    let output = cmd
        .output()
        .context("running git clone (is git installed?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git clone failed:\n{}", stderr.trim_end());
    }

    Ok(tmp)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parse_repo_root() {
        let r = parse_github_url("https://github.com/owner/repo").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo, "repo");
        assert!(r.branch.is_none());
        assert!(r.path.is_none());
    }

    #[test]
    fn parse_with_branch_and_path() {
        let r = parse_github_url("https://github.com/owner/repo/tree/main/commands").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo, "repo");
        assert_eq!(r.branch.as_deref(), Some("main"));
        assert_eq!(r.path.as_deref(), Some("commands"));
    }

    #[test]
    fn parse_blob_with_nested_path() {
        let r = parse_github_url("https://github.com/org/repo/blob/dev/a/b/c.md").unwrap();
        assert_eq!(r.owner, "org");
        assert_eq!(r.repo, "repo");
        assert_eq!(r.branch.as_deref(), Some("dev"));
        assert_eq!(r.path.as_deref(), Some("a/b/c.md"));
    }

    #[test]
    fn parse_no_scheme() {
        let r = parse_github_url("github.com/owner/repo").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo, "repo");
        assert!(r.branch.is_none());
        assert!(r.path.is_none());
    }

    #[test]
    fn parse_shorthand() {
        let r = parse_github_url("owner/repo").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo, "repo");
    }

    #[test]
    fn parse_strips_git_suffix() {
        let r = parse_github_url("owner/repo.git").unwrap();
        assert_eq!(r.repo, "repo");
    }

    #[test]
    fn parse_trailing_slash() {
        let r = parse_github_url("https://github.com/owner/repo/").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo, "repo");
        assert!(r.branch.is_none());
    }

    #[test]
    fn parse_tree_branch_only() {
        let r = parse_github_url("https://github.com/owner/repo/tree/main").unwrap();
        assert_eq!(r.branch.as_deref(), Some("main"));
        assert!(r.path.is_none());
    }

    #[test]
    fn parse_empty_fails() {
        assert!(parse_github_url("").is_err());
    }

    #[test]
    fn parse_missing_repo_fails() {
        assert!(parse_github_url("owner").is_err());
    }

    #[test]
    fn parse_non_github_fails() {
        assert!(parse_github_url("https://gitlab.com/owner/repo").is_err());
    }

    #[test]
    fn parse_missing_branch_after_tree_fails() {
        assert!(parse_github_url("https://github.com/owner/repo/tree").is_err());
    }

    #[test]
    fn parse_unexpected_segment_fails() {
        assert!(parse_github_url("https://github.com/owner/repo/wiki/page").is_err());
    }

    #[test]
    fn clone_url_format() {
        let r = parse_github_url("https://github.com/o/r").unwrap();
        assert_eq!(r.clone_url(), "https://github.com/o/r.git");
    }
}
