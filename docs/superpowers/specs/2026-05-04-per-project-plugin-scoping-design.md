# Per-Project Plugin Scoping (CCPM Feature B)

**Date:** 2026-05-04
**Status:** Design approved, ready for implementation plan
**Source:** Brainstormed via `/superpowers:brainstorming` against `CLAUDE.md`, `FEATURE_PLAN.md`, `docs/architecture.md`, and the Claude Code official docs.

## 1. Motivation

CCPM today toggles plugins by writing `enabledPlugins["plugin@marketplace"]` in `settings.json` files. The user has 55 plugins enabled in `~/.claude/settings.json` and ~19 projects under `~/Projects/`. The pain point: a user-scope plugin like `gitlab@claude-plugins-official` is globally enabled but should be off in github-only projects. Today the user manually edits each project's `.claude/settings.local.json` to override; CCPM cannot do this in one keystroke and does not even display the override correctly.

Two existing items in `FEATURE_PLAN.md` cover the work:
- **Feature B (Scope Selection)** — keystroke to choose which scope a toggle writes to.
- **Item #6 (Local override for user-scope plugins)** — discovery bug where user-scope plugins ignore CWD's project/local settings.

This spec combines both into one focused deliverable.

## 2. Research findings (Claude Code internals)

Verified against [code.claude.com/docs/en/settings](https://code.claude.com/docs/en/settings) and [code.claude.com/docs/en/discover-plugins](https://code.claude.com/docs/en/discover-plugins):

1. **`enabledPlugins` in `settings.json` is the only gate.** No other file (manifest, installed_plugins.json, cache) controls whether a plugin loads.
2. **Precedence:** Local > Project > User. A `false` at Local overrides a `true` at User. `enabledPlugins` is a boolean dict, so individual entries are subject to precedence (no array-merge).
3. **Runtime:** changes apply on next session, or `/reload-plugins` inside a session.
4. **`installed_plugins.json`** is purely an installation registry and is never consulted for the enable/disable question.
5. **Cross-scope override works:** a plugin installed at user scope CAN be disabled per-project via that project's `settings.json` or `settings.local.json`. This is the mechanism we exploit.
6. **`~/.claude/plugins/cache/`** is undocumented; a known troubleshooting workaround is `rm -rf` on it. Out of scope here unless testing reveals issues.

Conclusion: writing `enabledPlugins[id] = false` to `./.claude/settings.local.json` is sufficient to disable a globally-enabled plugin in the current project. No other files need touching.

## 3. Goals and non-goals

### In scope (this feature)

1. **Discovery fix**: for every plugin, populate `enabled_project` and `enabled_local` from CWD's `settings.json` / `settings.local.json` — including user-scope-installed plugins, which today are hardcoded to `(None, None)`.
2. **Approach-1 keybindings** (one keystroke per scope):
   - `Enter` and `l` → toggle Local (`./.claude/settings.local.json`)
   - `p` → toggle Project (`./.claude/settings.json`)
   - `u` → toggle User (`~/.claude/settings.json`)
3. **Effective-state UI**: list row `[+]/[-]` reflects effective state; details pane shows per-scope breakdown plus "winning" scope; row marker `↓` when a non-install-scope override is in play.
4. **Help overlay** updated with the four keybindings.

### Explicitly out of scope (deferred)

| Item | Reason |
|---|---|
| Bulk select / multi-toggle (Hygiene A) | User deprioritized in favor of B |
| Multi-project view scanning `~/Projects/*` (C) | Backlog |
| Profiles/presets (D) | Backlog |
| Walk-up project root discovery from CWD | "Scope 1" — deferred per user |
| `--project <path>` flag | "Scope 1" — deferred per user |
| `~/.config/ccpm/config.toml` for `projects_root` | Only needed when C lands |
| `Shift+l` / `d` to remove an override (3-state) | Add later if friction emerges |
| Cache invalidation under `~/.claude/plugins/cache/` | Undocumented; reach for only if testing surfaces a stale-state bug |
| Building the `ScopeSelectionMode` enum + Modal scope-picker dialog | Was planned in `FEATURE_PLAN.md` but never landed in source. We do **not** build it now — Approach 1 is simpler. Re-add as a clean addition if Approach 2 is wanted later. |

### Active project definition

For this feature: **active project = current working directory**, no walk-up, no flag. Same as today's behavior. Documented limitation: running CCPM from a subdirectory of a project will create `.claude/` at the wrong level. The Help overlay should note "Run from project root."

## 4. Design

### 4.1 Discovery fix (`src/plugin/discovery.rs`)

The `match install_scope` block at lines ~90-115 currently returns `(None, None)` for `Scope::User`. Change to:

```rust
let (plugin_enabled_project, plugin_enabled_local) = match install_scope {
    Scope::User => {
        // User-scope plugins now pick up CWD overrides
        (
            cwd_project_enabled.get(id).copied(),
            cwd_local_enabled.get(id).copied(),
        )
    }
    Scope::Project | Scope::Local => {
        // Unchanged: read from the plugin's own project_path for cross-project isolation
        if let Some(ref proj_path) = entry.project_path {
            let (proj_settings, local_settings) = project_settings_cache
                .entry(proj_path.clone())
                .or_insert_with(|| ConfigPaths::load_settings_from_project(proj_path));
            (
                proj_settings.as_ref().and_then(|s| s.enabled_plugins.get(id).copied()),
                local_settings.as_ref().and_then(|s| s.enabled_plugins.get(id).copied()),
            )
        } else {
            (
                cwd_project_enabled.get(id).copied(),
                cwd_local_enabled.get(id).copied(),
            )
        }
    }
};
```

The existing `Plugin::is_enabled()` (Local > Project > User) and `effective_scope()` work unchanged — they do the right thing once these fields are populated.

### 4.2 Toggle action (`src/app.rs`)

Replace the existing `toggle_plugin` (which writes to `install_scope`) with a scope-explicit variant:

```rust
pub fn toggle_at_scope(&self, plugin: &Plugin, scope: Scope) -> Result<bool> {
    let current_setting = match scope {
        Scope::User    => plugin.enabled_user,
        Scope::Project => plugin.enabled_project,
        Scope::Local   => plugin.enabled_local,
    };
    let new_state = match current_setting {
        Some(b) => !b,
        None    => !plugin.is_enabled(), // first press flips effective state
    };
    self.service.set_plugin_enabled(&plugin.id, scope, new_state)?;
    Ok(new_state)
}
```

The `None` branch is critical: pressing `l` on a plugin with no Local setting writes `Local = !is_enabled()`, so the first keystroke immediately flips the *effective* state — matching user intuition (gitlab is on; press `l`; gitlab is off here).

### 4.3 Keybindings (`src/main.rs`)

In `AppMode::Normal`:

| Key | Action |
|---|---|
| `Enter` | `toggle_at_scope(plugin, Scope::Local)` |
| `l` | `toggle_at_scope(plugin, Scope::Local)` |
| `p` | `toggle_at_scope(plugin, Scope::Project)` |
| `u` | `toggle_at_scope(plugin, Scope::User)` |

Other existing keybindings (Tab, search, help, etc.) unchanged.

### 4.4 UI changes

**List row** (`src/ui/plugin_list.rs`):

Today: `[U] [+] gitlab@claude-plugins-official`

After: `[U]↓ [-] gitlab@claude-plugins-official`

The `↓` marker is shown when any non-install-scope override is set:
- For a `Scope::User` install: marker on if `enabled_project.is_some()` or `enabled_local.is_some()`.
- For a `Scope::Project` install: marker on if `enabled_user.is_some()` or `enabled_local.is_some()`.
- For a `Scope::Local` install: marker on if `enabled_user.is_some()` or `enabled_project.is_some()`.

**Details pane** (`src/ui/details.rs`): add a Settings block —

```
Settings:
  User:     ✓ enabled
  Project:  – (no setting)
  Local:    ✗ disabled
  ─────────────────────
  Effective: DISABLED (Local override)
```

The "Effective" line uses the existing `effective_scope()` to determine which scope is winning. Format helpers:
- `Some(true)` → `✓ enabled`
- `Some(false)` → `✗ disabled`
- `None` → `– (no setting)`

**Status bar** (header): append `[overrides: N]` where N is the number of plugins in the **currently filtered list** (after search and scope filter applied) that have any non-install-scope setting. Cheap derivation from already-loaded data.

**Help overlay** (`src/ui/help.rs`): add the four-key block plus the "Run from project root" note.

### 4.5 What we explicitly do NOT build

`FEATURE_PLAN.md` and `CLAUDE.md` describe an in-progress `ScopeSelectionMode` enum, `AppMode::ScopeSelect`, and a modal scope picker. **None of these are present in the source today** — they were design notes that never landed. Approach 1 makes them unnecessary. If a future iteration wants Approach 2 (modal), it's a clean addition: an enum + an `AppMode` variant + a dispatch branch in `main.rs`. Don't ship the modal now to "preserve the option" — premature scaffolding rots.

## 5. Edge cases

| Case | Behavior |
|---|---|
| CWD has no `.claude/` directory | `set_plugin_enabled` → `fs::create_dir_all` already handles this; silent create. |
| CWD == `$HOME` | `./.claude/settings.local.json` resolves to `~/.claude/settings.local.json`. **Whether Claude Code reads this file is undocumented in the official docs we reviewed** (settings docs only describe `~/.claude/settings.json` for the User scope). MVP: write proceeds and the file is created; user is responsible for not running CCPM from `$HOME`. The Help overlay should mention "Run from project root, not `$HOME`." |
| CWD is a subdirectory of a project | Creates `.claude/` at the wrong level (subdirectory). Known limitation; Help overlay notes "Run from project root." Walk-up is deferred to Scope 1. |
| Phantom plugin (in settings, not installed) | `discovery.rs:149-177` already builds Plugin entries for these from settings; toggle works on the id like any other. |
| Two CCPM processes overlap | Existing `LockFileGuard` + stale-PID detection. `LockConflict` is surfaced in the TUI as today. |
| `settings.json` has top-level keys other than `enabledPlugins` | Preserved by `Settings { #[serde(flatten)] other }` in `config.rs:13`. Atomic write doesn't clobber unrelated keys. |
| Pressing `l` twice (false → true) | Override stays present; effective state flips. Removing the override is deferred (manual edit or future `Shift+l`/`d`). |
| Pressing `u` while a Local override exists | User scope is updated; effective state may not change (Local still wins). Details pane makes this explicit by showing all three scopes. |

## 6. Testing

### Unit tests

`src/plugin/discovery.rs`:
- User-scope plugin + CWD has `Local: false` for that id → resulting `Plugin.enabled_local == Some(false)` and `is_enabled() == false`.
- User-scope plugin + CWD has `Project: false` → `enabled_project == Some(false)`.
- User-scope plugin + no CWD overrides → `enabled_project == None`, `enabled_local == None`, behavior unchanged.
- Project/Local-scope plugin installed in another project, CWD has overrides for the same id → CWD overrides ignored (cross-project isolation preserved).

`src/app.rs`:
- `toggle_at_scope` `None` branch flips effective state on first press.
- `toggle_at_scope` `Some(b)` branch flips the boolean.
- Toggle on each of `Scope::User | Project | Local` writes to the correct file.

`src/plugin/operations.rs`: existing tests cover the `set_plugin_enabled` write path (`test_enable_disable_plugin` etc.). Add: toggling Local on a clean directory creates `./.claude/settings.local.json` with the right contents (verifies `fs::create_dir_all` + atomic write end-to-end for the new flow).

### Integration tests (`tests/integration.rs`)

- Fixture: project with `~/.claude/settings.json` enabling `gitlab@x`, project's `.claude/settings.local.json` setting `gitlab@x: false`. Run `ccpm list --debug`; assert effective state is disabled and the debug output reports `enabled_local=Some(false)`.
- CLI parity: `ccpm enable <id> --scope local` and `ccpm disable <id> --scope local` write to `settings.local.json`. (CLI already supports `--scope` via `ScopeArg` in `src/cli/mod.rs:64`; just need new test cases covering the per-scope write paths if not already covered.)

### TUI keybindings

Drive the dispatch logic via `app.rs` unit tests rather than terminal I/O. The keybinding-to-action mapping is the testable part; the rendering is exercised by hand.

## 7. Files touched

| File | Change |
|---|---|
| `src/plugin/discovery.rs` | The `Scope::User` arm in the match no longer returns `(None, None)`; pulls from `cwd_project_enabled` / `cwd_local_enabled`. New tests. |
| `src/app.rs` | Replace the `set_plugin_enabled`/`toggle_plugin` call (line ~175) with a `toggle_at_scope(plugin, Scope)` flow. New unit tests for the toggle helper. |
| `src/main.rs` | Key handlers for `Enter / l / p / u` in `AppMode::Normal` dispatching to `toggle_at_scope` with the appropriate scope. |
| `src/ui/plugin_list.rs` | `↓` override marker on rows where any non-install-scope setting is set. |
| `src/ui/details.rs` | Settings block (per-scope breakdown) + Effective line. |
| `src/ui/help.rs` | Document `Enter / l / p / u` keybindings + "Run from project root, not `$HOME`." |
| `tests/integration.rs` | Override-fixture test + per-scope CLI test cases. |

## 8. Documentation updates (per `CLAUDE.md` rules)

After merge:
- `CLAUDE.md` — update **CLI Commands** and **Architecture > Key Concepts** with the new keybindings and behavior.
- `docs/architecture.md` — already flagged outdated; refresh Plugin struct documentation and the Settings Loading Strategy block to note that user-scope plugins now also pick up CWD overrides.
- `FEATURE_PLAN.md` — move Feature B and item #6 to **Completed**; record the Approach-1 decision.
- `README.md` — keybindings table changed (user-visible).
- `CLAUDE.local.md` — append low-level implementation notes.
- Delete or move the old `.claude/ccpm-task-scope.md` per the temporal-artifacts cleanup policy.

## 9. References

- [Claude Code: Settings](https://code.claude.com/docs/en/settings) — precedence hierarchy, `enabledPlugins` semantics.
- [Claude Code: Discover & manage plugins](https://code.claude.com/docs/en/discover-plugins) — `/reload-plugins`, install vs enable.
- `CLAUDE.md` — project conventions and documentation rules.
- `FEATURE_PLAN.md` — Features B (Scope Selection) and #6 (Local override for user-scope plugins).
- `docs/architecture.md` — current data model and discovery strategy.
