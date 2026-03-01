# ccpick

Per-project Claude Code extension manager. ([中文](README_zh.md))

## The problem

Claude Code commands, skills, and plugins either live in `~/.claude/` (loaded globally into every session) or are manually copied into each project's `.claude/` directory. Global loading means a `frontend-design` skill useful only in frontend/fullstack projects appears everywhere — and every loaded extension consumes context window (even though skills use progressive disclosure, we still want to exclude unrelated context). Manual copying is tedious, creates multiple copies in your computer, and can't be kept in sync.

## Install

Pre-requisites:
- Git
- Rust, Cargo (https://www.rust-lang.org/tools/install)

```bash
git clone https://github.com/zijian-yi/ccpick
cd ccpick
cargo install --path .
```

## What ccpick does

ccpick maintains a central library at `~/.claude/ccpick/` and keeps the global directories minimal. For each project, you pick which commands and skills to enable — ccpick symlinks them into the project's `.claude/` directory, ensuring a single source of truth that stays in sync.

ccpick also manages Claude Code's plugin mechanism (which is global by default), writing enable/disable states to `.claude/settings.local.json`. All selections are recorded in a manifest for reproducibility.

## Quick start

```bash
# 1. Move existing global extensions into the library (one-time)
ccpick tidy

# 2. Pick extensions for the current project
ccpick init

# 3. Later, update selections
ccpick edit

# 4. Re-apply from manifest (e.g., after git clone)
ccpick sync
```

### Templates

Save and reuse configurations across projects:

```bash
ccpick template save backend        # save current project as template
ccpick template apply backend       # apply to another project
```

See [USAGE.md](USAGE.md) for the full command reference.

## How it works

1. `ccpick tidy` moves files from `~/.claude/commands/` and `~/.claude/skills/` into `~/.claude/ccpick/{commands,skills}/` (the library).
2. `ccpick init` scans the library, presents an interactive checkbox picker, then creates absolute symlinks from the project's `.claude/commands/` and `.claude/skills/` back to the library.
3. For plugins, ccpick reads the installed plugin registry (`~/.claude/plugins/installed_plugins.json`), lets you pick which to enable or disable at a project level, and writes the selections to `.claude/settings.local.json`.
4. All selections are saved to `.claude/ccpick.json` (the manifest). Running `ccpick sync` re-applies from this file without prompting.
5. A managed block in `.claude/.gitignore` keeps symlinked entries out of version control while preserving user-added ignores.

Each project's `.claude/` directory is independent. No global state is mutated at runtime, so concurrent sessions across projects are safe.


## Acknowledgments

- [trailofbits/claude-code-config](https://github.com/trailofbits/claude-code-config)