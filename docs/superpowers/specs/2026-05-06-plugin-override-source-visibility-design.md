# Plugin Override-Source Visibility

**Date:** 2026-05-06
**Status:** Approved (pending implementation)

## Problem

A user-installed plugin (registry `scope: "user"`) can be overridden by a per-project `enabledPlugins` flag in `<project>/.claude/settings.json` or `<project>/.claude/settings.local.json`. CCPM correctly reads these overrides and resolves the effective state — that part works. The gap is **visibility**: the details pane shows `Local: enabled` but not *which project's* `settings.local.json` contains that flag.

Concrete example. `chrome-devtools-mcp@claude-plugins-official` is registered at user scope, but `Ternv3/.claude/settings.local.json` pins it to `true`. Today CCPM shows:

```
Settings:
  User:    enabled
  Project: (no setting)
  Local:   enabled
Effective: ENABLED (Local)
```

A reader sitting in `~/Projects/ccpm` cannot tell from this view that the `Local: enabled` flag is sourced from a Ternv3 file, not from this project. The information has to be found by greping `~/Projects/**/.claude/settings*.json`. The same gap exists in `ccpm info <id>` CLI output.

This spec closes that gap.

## Goal

When a `Project` or `Local` flag exists for a plugin, surface the **file path that contains that flag** directly next to the value — in both the TUI details pane and the `ccpm info` CLI output. No filesystem grep required to answer "where does this override live?"

## Approach

Append the source file path to each `Settings:` row that has a value at `Project` or `Local` scope. Reuse the existing details pane layout — no new screen, no inline per-row badges in the plugin list. The `User` row is never annotated because its source is always `~/.claude/settings.json` and that's not what users are confused about.

The path is derivable at render time from `Plugin.project_path` (for project/local-scope installs) or from CWD (for user-scope installs with a CWD override). No new fields on `Plugin`; one new helper method.

### Out of scope

- Detection of orphan caches, orphan marketplaces, missing-installPath, half-uninstalled plugins (this is the larger "doctor" scope, deferred — see `T2` discussion in brainstorming).
- Any destructive ops or shelling out to `claude plugin uninstall` (deferred — see `T3`).
- Inline per-row override badges in the plugin list (option C from brainstorming, recorded as a fallback if this spec under-delivers; see project memory).
- `ccpm list` default-output column changes — `list` is a table; the natural carrier for per-plugin detail is `ccpm info`.
- Annotating the `User` Settings row.

## Changes

### `src/plugin/mod.rs`

Add one helper on `Plugin`:

```rust
use std::path::{Path, PathBuf};

impl Plugin {
    /// Path to the settings file that supplies this plugin's flag at the given scope.
    ///
    /// Returns `None` if no flag exists at the requested scope, or for `Scope::User`
    /// (the user-scope flag always lives in `~/.claude/settings.json` — uninteresting,
    /// not annotated by callers).
    ///
    /// For project/local-scope installs the project directory is `self.project_path`.
    /// For user-scope installs with a CWD override (project_path is None) the project
    /// directory is `cwd`.
    pub fn project_settings_source(&self, scope: Scope, cwd: &Path) -> Option<PathBuf> {
        let has_value = match scope {
            Scope::User => return None,
            Scope::Project => self.enabled_project.is_some(),
            Scope::Local => self.enabled_local.is_some(),
        };
        if !has_value {
            return None;
        }

        let project_dir = self.project_path.as_deref().unwrap_or(cwd);
        let file_name = match scope {
            Scope::Project => "settings.json",
            Scope::Local => "settings.local.json",
            Scope::User => unreachable!(),
        };
        Some(project_dir.join(".claude").join(file_name))
    }
}
```

### `src/ui/details.rs`

In the existing `Settings:` block (currently `lines.push(...)` for User / Project / Local rows), when rendering the Project and Local rows, additionally append a ` · <path>` suffix in `DarkGray` whenever `project_settings_source(scope, cwd)` returns `Some`.

After:

```
Settings:
  User:    enabled
  Project: (no setting)
  Local:   enabled  · ~/Projects/tern/Ternv3/.claude/settings.local.json
Effective: ENABLED (Local)
```

CWD comes from `std::env::current_dir()`. Path display uses the existing home-relative format (replace `$HOME` prefix with `~`), reusing the same style as `Plugin::project_path_display()`.

### `src/cli/mod.rs` (`info` subcommand)

Mirror the same per-row suffix in the plain-text `info` output. No ANSI styling. Same path format.

## Edge cases

- **`enabled_*` is `None`** → row renders `(no setting)`, no annotation. Correct.
- **Project/local-scope install with `project_path: None`** (legacy data pre-projectPath fix) → falls back to CWD. The displayed path may be wrong for that legacy case, but it's consistent with how the rest of CCPM already resolves these installs.
- **CWD has no `.claude/` directory but a User-scope plugin's `enabled_local` is `Some`** → shouldn't be reachable in normal flow (the flag wouldn't exist if the file didn't exist), but if it happens, the displayed path points to a non-existent file. Acceptable — surfaces inconsistency rather than hiding it.
- **Multiple overrides at once** (e.g. `Project: false` AND `Local: true`) → both rows are annotated independently. Each row tells its own story; the `Effective:` line still tells you which won.

## Tests

### Unit tests in `src/plugin/mod.rs`

1. **`test_project_settings_source_local_install_with_project_path`** — Construct a `Plugin` with `install_scope: Local`, `project_path: Some("/proj")`, `enabled_local: Some(true)`. Assert `project_settings_source(Scope::Local, &any_cwd)` returns `Some(/proj/.claude/settings.local.json)`.

2. **`test_project_settings_source_user_install_with_cwd_override`** — Construct a `Plugin` with `install_scope: User`, `project_path: None`, `enabled_local: Some(false)`. Pass a CWD like `/cwd`. Assert source is `Some(/cwd/.claude/settings.local.json)`.

3. **`test_project_settings_source_user_scope_returns_none`** — For any plugin, `project_settings_source(Scope::User, &any_cwd)` returns `None`.

4. **`test_project_settings_source_no_value_returns_none`** — When `enabled_project` is `None`, `project_settings_source(Scope::Project, ...)` returns `None`.

5. **`test_project_settings_source_project_scope_uses_settings_json`** — Distinguishes `Scope::Project` (yields `settings.json`) from `Scope::Local` (yields `settings.local.json`).

### Unit tests in `src/ui/details.rs`

6. **`test_details_renders_path_on_local_row_when_set`** — Render a `Plugin` whose `enabled_local` is `Some(true)`. Assert the `Local:` row in the rendered output contains the `· ` separator and the file path.

7. **`test_details_skips_path_on_no_setting_rows`** — Render a `Plugin` whose `enabled_project` is `None`. Assert the `Project:` row does not contain ` · `.

8. **`test_details_skips_path_on_user_row`** — Render a `Plugin` whose `enabled_user` is `Some(true)`. Assert the `User:` row does not contain ` · ` (User is never annotated).

### Integration test in `tests/integration.rs`

9. **`test_info_command_shows_override_source`** — Set up a tempdir with installed_plugins.json declaring a user-scope plugin, and a separate "project" tempdir with `.claude/settings.local.json` overriding it. Invoke `ccpm info <id>` from the project tempdir. Assert stdout contains the `· <path>` suffix on the `Local` row.

## Risk

- **Low.** Additive. No existing code paths change behavior; one new helper, one render-time string concat in two places.
- No persisted state changes. No effect on `installed_plugins.json` or any settings file.
- No effect on plugin enable/disable logic.
