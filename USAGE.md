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

## File layout

| Path | Purpose |
|------|---------|
| `~/.claude/ccpick/commands/` | Command library |
| `~/.claude/ccpick/skills/` | Skill library |
| `~/.claude/ccpick/templates/` | Saved templates |
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
