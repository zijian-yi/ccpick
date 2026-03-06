# ccpick usage

Per-project Claude Code extension manager. Keeps `~/.claude/commands/` and `~/.claude/skills/` minimal globally, and symlinks selected items from a central library into each project's `.claude/` directory.

## Commands

### `ccpick tidy`

Move global commands, skills, and plugin defaults into the ccpick library (`~/.claude/ccpick/`). Run once to populate the library.

```
ccpick tidy [OPTIONS]
```

| Flag | Effect |
|------|--------|
| `--commands` | Only tidy commands |
| `--skills` | Only tidy skills |
| `--plugins` | Only tidy plugins |

When no flags are given, all three categories are tidied.

### `ccpick install`

Install commands and skills from a GitHub repository into the ccpick library (or other targets).

```
ccpick install <URL> [OPTIONS]
```

| Flag | Effect |
|------|--------|
| `-g`, `--global` | Install to `~/.claude/` (global) |
| `-l`, `--local` | Install to `.claude/` (current project only) |
| `--branch <BRANCH>` | Git branch (overrides branch parsed from URL) |

Without `-g` or `-l`, items install to the ccpick library (`~/.claude/ccpick/`).

**URL formats:**

| Pattern | Example |
|---------|---------|
| Full URL | `https://github.com/owner/repo` |
| No scheme | `github.com/owner/repo` |
| Shorthand | `owner/repo` |
| Specific path | `https://github.com/owner/repo/tree/main/commands` |
| Single file | `https://github.com/owner/repo/blob/main/commands/config.md` |

**Repo root** (no sub-path in URL): scans `commands/` and `skills/` directories and opens an interactive picker.

**Specific path** (sub-path in URL): auto-detects whether the path is a command (`.md` file) or skill (directory with `skill.md`) and installs it directly.

If the destination already exists, the item is skipped.

### `ccpick init`

Set up the current project. Without `--template`, opens an interactive picker for commands, skills, and plugins. Creates symlinks in `.claude/commands/` and `.claude/skills/`, writes plugin settings to `.claude/settings.local.json`, and saves selections to `.claude/ccpick.json`.

```
ccpick init [OPTIONS]
```

| Flag | Effect |
|------|--------|
| `--template <NAME>` | Apply a saved template directly (non-interactive) |

With `--template`, the template's selections are applied without prompting. Use `ccpick edit` afterward to adjust.

### `ccpick edit`

Re-open the interactive picker with current selections pre-checked. Requires an existing `.claude/ccpick.json`.

```
ccpick edit
```

### `ccpick sync`

Non-interactive. Re-applies symlinks and plugin settings from the existing `.claude/ccpick.json`. Warns about items missing from the library.

```
ccpick sync
```

### `ccpick template`

Manage reusable configuration templates stored at `~/.claude/ccpick/templates/`.

#### `ccpick template save <NAME>`

Save the current project's `.claude/ccpick.json` as a named template.

```
ccpick template save backend
```

Requires an existing manifest in the current project.

#### `ccpick template create [NAME]`

Interactively pick commands, skills, and plugins, then save as a template. Does not modify the current project.

```
ccpick template create           # prompts for name at the end
ccpick template create backend   # pre-fills "backend" as default name
```

The name is always confirmed via prompt, with the CLI argument as the default value.

#### `ccpick template apply <NAME>`

Apply a template to the current project. Shortcut for `ccpick init --template <NAME>`.

```
ccpick template apply backend
```

#### `ccpick template edit <NAME>`

Re-open the interactive picker with the template's current selections pre-checked. Saves changes back to the same template.

```
ccpick template edit backend
```

#### `ccpick template list`

List all saved template names.

```
ccpick template list
```

#### `ccpick template delete <NAME>`

Delete a saved template.

```
ccpick template delete backend
```

### `ccpick guide`

Manage CLAUDE.md and AGENTS.md files using templates and presets. Guide templates contain `{{ slot_name }}` placeholders that are filled by slot presets. Top-level presets are complete, ready-to-use content. Both `CLAUDE.md` and `AGENTS.md` receive identical content.

#### `ccpick guide template list`

List guide templates.

```
ccpick guide template list
```

#### `ccpick guide template create <NAME>`

Create a new guide template in `$EDITOR`.

```
ccpick guide template create base
```

Templates use `{{ slot_name }}` placeholders on their own lines. Slot names: `[a-zA-Z0-9_]+`. Duplicate slots are an error.

#### `ccpick guide template edit <NAME>`

Edit an existing guide template in `$EDITOR`.

```
ccpick guide template edit base
```

#### `ccpick guide template delete <NAME>`

Delete a guide template.

```
ccpick guide template delete base
```

#### `ccpick guide preset list [SLOT]`

List presets. Without a slot argument, lists top-level presets and all slot presets. With a slot argument, lists presets for that slot.

```
ccpick guide preset list            # all presets
ccpick guide preset list language   # presets for the "language" slot
```

#### `ccpick guide preset create <NAME>`

Create a top-level preset in `$EDITOR`.

```
ccpick guide preset create rust-backend
```

#### `ccpick guide preset create <NAME> --from-template <TEMPLATE>`

Fill a template interactively and save the result as a top-level preset.

```
ccpick guide preset create rust-backend --from-template base
```

#### `ccpick guide preset create <SLOT> <NAME>`

Create a slot preset in `$EDITOR`.

```
ccpick guide preset create language rust
```

#### `ccpick guide preset edit <NAME>`

Edit a top-level preset in `$EDITOR`.

```
ccpick guide preset edit rust-backend
```

#### `ccpick guide preset edit <SLOT> <NAME>`

Edit a slot preset in `$EDITOR`.

```
ccpick guide preset edit language rust
```

#### `ccpick guide preset delete <NAME>`

Delete a top-level preset.

```
ccpick guide preset delete rust-backend
```

#### `ccpick guide preset delete <SLOT> <NAME>`

Delete a slot preset.

```
ccpick guide preset delete language rust
```

#### `ccpick guide apply [PRESET]`

Apply a top-level preset, writing identical content to `CLAUDE.md` and `AGENTS.md` in the project root. If no preset is given, opens an interactive picker. Prompts before overwriting existing files.

```
ccpick guide apply rust-backend
ccpick guide apply                  # opens picker
```

#### `ccpick guide compose [TEMPLATE]`

Compose from a template by interactively selecting a preset for each slot, then write `CLAUDE.md` and `AGENTS.md`. If no template is given, opens an interactive picker. Prompts before overwriting existing files.

```
ccpick guide compose base
ccpick guide compose                # opens picker
```

#### `ccpick guide show <NAME>`

Preview rendered output without writing files. Looks up the name as a preset first, then as a template.

```
ccpick guide show rust-backend
ccpick guide show base
```

### `ccpick completions`

Generate shell completion scripts.

```
ccpick completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

**Setup examples:**

```bash
# Zsh (add to ~/.zshrc)
eval "$(ccpick completions zsh)"

# Bash (add to ~/.bashrc)
eval "$(ccpick completions bash)"

# Fish (run once)
ccpick completions fish > ~/.config/fish/completions/ccpick.fish
```

## File layout

| Path | Purpose |
|------|---------|
| `~/.claude/ccpick/commands/` | Command library |
| `~/.claude/ccpick/skills/` | Skill library |
| `~/.claude/ccpick/templates/` | Saved templates |
| `~/.claude/ccpick/guide/templates/` | Guide templates |
| `~/.claude/ccpick/guide/presets/` | Top-level guide presets |
| `~/.claude/ccpick/guide/presets/{slot}/` | Slot guide presets |
| `.claude/ccpick.json` | Project manifest |
| `.claude/commands/` | Symlinks to library commands |
| `.claude/skills/` | Symlinks to library skills |
| `.claude/settings.local.json` | Plugin settings |

## Manifest format

```json
{
  "version": 1,
  "commands": ["trailofbits/config.md"],
  "skills": ["review"],
  "plugins": {
    "org/plugin-name": true,
    "org/disabled-plugin": false
  }
}
```

Templates use the same format, stored as `~/.claude/ccpick/templates/<name>.json`.

## Template name rules

Names may contain alphanumeric characters, hyphens, and underscores. No spaces or path separators.
