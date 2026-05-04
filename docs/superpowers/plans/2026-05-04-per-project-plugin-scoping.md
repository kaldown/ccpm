# Per-Project Plugin Scoping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let CCPM users override plugin enabled state per-project from the TUI with one keystroke (`l`/`Enter`/`Space`) and surface effective Local > Project > User precedence in the UI. Closes Feature B and bug #6 from `FEATURE_PLAN.md`.

**Architecture:** Two layers. (a) Discovery-layer fix in `src/plugin/discovery.rs` so user-scope plugins read CWD `.claude/settings.json` and `.claude/settings.local.json` and populate `enabled_project` / `enabled_local`. (b) Action layer: new `PluginService::toggle_at_scope(plugin, scope)` and corresponding `App` methods, wired to per-scope keybindings (`l`, `p`, `u`) plus repurposed defaults (`Enter`, `Space`, `e`, `d` → Local). Detail modal moves from `Enter` to `i`.

**Tech Stack:** Rust 1.70+, ratatui (TUI), crossterm (terminal events), assert_cmd + predicates (CLI integration tests), tempfile (test fixtures), serde + serde_json (settings I/O), fs2 (file locking, already in use).

**Spec:** `docs/superpowers/specs/2026-05-04-per-project-plugin-scoping-design.md`. Implementation may diverge slightly: detail modal moves to `i` because the spec's `Enter → toggle Local` collides with the existing `Enter → show detail modal` binding. All other spec items hold.

**Verification commands** (run after every task; CCPM-standard):

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
```

Do not advance to the next task while any of these fail.

---

## Task 1: Fix discovery — load CWD overrides for user-scope plugins

**Why:** Today `discovery.rs` returns `(None, None)` for `Scope::User` plugins, so the UI cannot show a user-scope plugin disabled in the current project. Bug #6.

**Files:**
- Modify: `src/plugin/discovery.rs:90-95` (the `match install_scope` block — `Scope::User` arm)
- Test: `src/plugin/discovery.rs` (existing `mod tests` block at the bottom)

**Note on testing:** `PluginDiscovery::with_paths` accepts a `ConfigPaths` whose `local_dir` can point at any absolute path, so tests don't need to mutate `env::current_dir()`. Reuse the existing helpers `create_test_settings` and `create_test_local_settings`.

- [ ] **Step 1: Write the failing test**

Append to the `mod tests` block in `src/plugin/discovery.rs` (after the existing `test_load_settings_from_nonexistent_project` test):

```rust
fn write_user_settings(user_dir: &Path, plugins: &[(&str, bool)]) {
    let mut enabled = serde_json::Map::new();
    for (id, on) in plugins {
        enabled.insert(id.to_string(), serde_json::Value::Bool(*on));
    }
    let json = serde_json::json!({ "enabledPlugins": enabled });
    fs::write(user_dir.join("settings.json"), serde_json::to_string_pretty(&json).unwrap())
        .unwrap();
}

fn write_user_installed(user_dir: &Path, id: &str, scope: &str, install_path: &Path) {
    let plugins_dir = user_dir.join("plugins");
    fs::create_dir_all(&plugins_dir).unwrap();
    let json = serde_json::json!({
        "version": 2,
        "plugins": {
            id: [{
                "scope": scope,
                "installPath": install_path,
                "version": "1.0.0",
                "installedAt": "2026-01-01T00:00:00Z",
                "lastUpdated": "2026-01-01T00:00:00Z"
            }]
        }
    });
    fs::write(
        plugins_dir.join("installed_plugins.json"),
        serde_json::to_string_pretty(&json).unwrap(),
    )
    .unwrap();
}

#[test]
fn test_user_scope_plugin_picks_up_cwd_local_override() {
    let user_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();
    let install_path = TempDir::new().unwrap();

    write_user_settings(user_dir.path(), &[("test@marketplace", true)]);
    write_user_installed(user_dir.path(), "test@marketplace", "user", install_path.path());

    create_test_local_settings(project_dir.path(), &[("test@marketplace", false)]);

    let paths = ConfigPaths {
        user_dir: user_dir.path().to_path_buf(),
        local_dir: project_dir.path().join(".claude"),
    };

    let plugins = PluginDiscovery::with_paths(paths).discover_all().unwrap();
    let plugin = plugins
        .iter()
        .find(|p| p.id == "test@marketplace")
        .expect("user-scope plugin should be discovered");

    assert_eq!(plugin.enabled_user, Some(true));
    assert_eq!(
        plugin.enabled_local,
        Some(false),
        "CWD local override must populate enabled_local for user-scope plugins"
    );
    assert!(!plugin.is_enabled(), "Local=false must override User=true");
}

#[test]
fn test_user_scope_plugin_picks_up_cwd_project_override() {
    let user_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();
    let install_path = TempDir::new().unwrap();

    write_user_settings(user_dir.path(), &[("test@marketplace", true)]);
    write_user_installed(user_dir.path(), "test@marketplace", "user", install_path.path());

    create_test_settings(project_dir.path(), &[("test@marketplace", false)]);

    let paths = ConfigPaths {
        user_dir: user_dir.path().to_path_buf(),
        local_dir: project_dir.path().join(".claude"),
    };

    let plugins = PluginDiscovery::with_paths(paths).discover_all().unwrap();
    let plugin = plugins
        .iter()
        .find(|p| p.id == "test@marketplace")
        .unwrap();

    assert_eq!(plugin.enabled_project, Some(false));
    assert_eq!(plugin.enabled_local, None);
    assert!(!plugin.is_enabled(), "Project=false must override User=true");
}

#[test]
fn test_user_scope_plugin_no_cwd_overrides_unchanged() {
    let user_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap(); // no .claude/ inside
    let install_path = TempDir::new().unwrap();

    write_user_settings(user_dir.path(), &[("test@marketplace", true)]);
    write_user_installed(user_dir.path(), "test@marketplace", "user", install_path.path());

    let paths = ConfigPaths {
        user_dir: user_dir.path().to_path_buf(),
        local_dir: project_dir.path().join(".claude"),
    };

    let plugins = PluginDiscovery::with_paths(paths).discover_all().unwrap();
    let plugin = plugins
        .iter()
        .find(|p| p.id == "test@marketplace")
        .unwrap();

    assert_eq!(plugin.enabled_user, Some(true));
    assert_eq!(plugin.enabled_project, None);
    assert_eq!(plugin.enabled_local, None);
    assert!(plugin.is_enabled());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --lib plugin::discovery -- test_user_scope_plugin
```

Expected: 3 tests fail. The first two fail because `enabled_local`/`enabled_project` come back `None`. The third may pass coincidentally — keep it as a regression guard.

- [ ] **Step 3: Apply the discovery fix**

In `src/plugin/discovery.rs`, change the `Scope::User` arm of the match (lines ~90-95). Replace this block:

```rust
let (plugin_enabled_project, plugin_enabled_local) = match install_scope {
    Scope::User => {
        // User scope: no project/local settings apply
        (None, None)
    }
    Scope::Project | Scope::Local => {
```

with:

```rust
let (plugin_enabled_project, plugin_enabled_local) = match install_scope {
    Scope::User => {
        // User-scope plugins still pick up CWD project/local overrides
        // (Local > Project > User precedence applies in is_enabled()).
        (
            cwd_project_enabled.get(id).copied(),
            cwd_local_enabled.get(id).copied(),
        )
    }
    Scope::Project | Scope::Local => {
```

The `Scope::Project | Scope::Local` arm is unchanged — cross-project isolation continues to read from the plugin's own `project_path`.

- [ ] **Step 4: Run tests to verify pass**

```bash
cargo test --lib plugin::discovery
```

Expected: all tests pass, including the three new ones and the existing `test_cross_project_settings_isolation`.

- [ ] **Step 5: Run full test suite + lints**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
```

Expected: green on all three.

- [ ] **Step 6: Commit**

```bash
git add src/plugin/discovery.rs
git commit -m "Load CWD overrides for user-scope plugins in discovery

Closes the FEATURE_PLAN #6 gap: user-scope plugins now populate
enabled_project / enabled_local from the current working directory
.claude/settings.json and .claude/settings.local.json. The existing
is_enabled() precedence (Local > Project > User) does the right thing
once these fields are non-None.

Cross-project isolation for project- and local-scope installs is
preserved (they continue to read from the plugin's own project_path)."
```

---

## Task 2: Add `PluginService::toggle_at_scope` (the action primitive)

**Why:** Today `PluginService::toggle_plugin(plugin)` writes to `plugin.install_scope`. The new feature needs an explicit-scope variant that handles the "no setting yet" case correctly: first press must flip the *effective* state, not toggle from a default.

**Files:**
- Modify: `src/plugin/operations.rs` (add `toggle_at_scope` near the existing `toggle_plugin` at line ~76)
- Test: `src/plugin/operations.rs` (existing `mod tests` block)

- [ ] **Step 1: Write failing tests**

Append to `mod tests` in `src/plugin/operations.rs` (the existing `use super::*;` at the top of the test module already brings `Plugin`, `Scope`, and `Settings` into scope):

```rust
fn make_plugin(id: &str, install_scope: Scope) -> Plugin {
    Plugin {
        id: id.to_string(),
        name: id.split('@').next().unwrap().to_string(),
        marketplace: id.split('@').nth(1).unwrap_or("unknown").to_string(),
        description: None,
        version: None,
        author: None,
        install_scope,
        install_path: None,
        project_path: None,
        is_current_project: true,
        enabled_user: None,
        enabled_project: None,
        enabled_local: None,
        installed_at: None,
        last_updated: None,
    }
}

#[test]
fn test_toggle_at_scope_none_branch_flips_effective_state() {
    let (_temp, service) = setup_test_env();
    let mut plugin = make_plugin("p@m", Scope::User);
    plugin.enabled_user = Some(true); // effective: enabled

    // Local is None; first press should write Local=false (flips effective state)
    let new_state = service.toggle_at_scope(&plugin, Scope::Local).unwrap();
    assert_eq!(new_state, false);

    let written = serde_json::from_str::<Settings>(
        &fs::read_to_string(service.paths.local_settings()).unwrap(),
    )
    .unwrap();
    assert_eq!(written.enabled_plugins.get("p@m"), Some(&false));
}

#[test]
fn test_toggle_at_scope_none_branch_with_no_user_setting() {
    let (_temp, service) = setup_test_env();
    let plugin = make_plugin("p@m", Scope::User); // no settings anywhere

    // is_enabled() == false; first press should write Local=true
    let new_state = service.toggle_at_scope(&plugin, Scope::Local).unwrap();
    assert_eq!(new_state, true);
}

#[test]
fn test_toggle_at_scope_some_branch_flips_boolean() {
    let (_temp, service) = setup_test_env();
    let mut plugin = make_plugin("p@m", Scope::User);
    plugin.enabled_local = Some(true);

    let new_state = service.toggle_at_scope(&plugin, Scope::Local).unwrap();
    assert_eq!(new_state, false);
}

#[test]
fn test_toggle_at_scope_writes_to_correct_file_per_scope() {
    let (_temp, service) = setup_test_env();
    let plugin = make_plugin("p@m", Scope::User);

    service.toggle_at_scope(&plugin, Scope::User).unwrap();
    assert!(service.paths.user_settings().exists());

    service.toggle_at_scope(&plugin, Scope::Project).unwrap();
    assert!(service.paths.project_settings().exists());

    service.toggle_at_scope(&plugin, Scope::Local).unwrap();
    assert!(service.paths.local_settings().exists());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --lib plugin::operations -- toggle_at_scope
```

Expected: 4 tests fail with "no method named `toggle_at_scope` found for struct `PluginService`".

- [ ] **Step 3: Implement `toggle_at_scope`**

In `src/plugin/operations.rs`, add this method to `impl PluginService` (after the existing `toggle_plugin` method):

```rust
/// Toggle a plugin's enabled state in a specific scope.
///
/// If that scope has no setting yet, first press flips the effective state
/// (so the user immediately sees a change). If a setting exists, flips it.
pub fn toggle_at_scope(&self, plugin: &Plugin, scope: Scope) -> Result<bool> {
    let current_setting = match scope {
        Scope::User => plugin.enabled_user,
        Scope::Project => plugin.enabled_project,
        Scope::Local => plugin.enabled_local,
    };
    let new_state = match current_setting {
        Some(b) => !b,
        None => !plugin.is_enabled(),
    };
    self.set_plugin_enabled(&plugin.id, scope, new_state)?;
    Ok(new_state)
}
```

- [ ] **Step 4: Run tests to verify pass**

```bash
cargo test --lib plugin::operations
```

Expected: all tests pass, including the 4 new ones.

- [ ] **Step 5: Lints**

```bash
cargo clippy -- -D warnings && cargo fmt --check
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add src/plugin/operations.rs
git commit -m "Add PluginService::toggle_at_scope for explicit per-scope toggling

The existing toggle_plugin always writes to plugin.install_scope; the new
helper takes an explicit Scope and handles the 'no setting yet' case by
flipping the effective state on first press. This is the primitive the
new keybindings (l/p/u and the repurposed Enter/Space/e/d) call into."
```

---

## Task 3: Add scope-aware action methods on `App`

**Why:** `app.rs` exposes `toggle_selected_plugin`, `enable_selected_plugin`, `disable_selected_plugin` — all hardcoded to `install_scope`. We add scope-explicit variants and update local plugin state mirroring on success.

**Files:**
- Modify: `src/app.rs` (add three new methods; keep the old ones for now — they'll be removed in Task 4 once main.rs is updated)

**Note:** No new unit tests in this task — `App` is integration-tested through CLI/TUI behavior. The underlying logic is already covered by Task 2's `toggle_at_scope` tests. Adding `App` unit tests would require breaking the implicit `PluginService::new()` dependency, which is out of scope.

- [ ] **Step 1: Add new scope-aware methods to `App`**

In `src/app.rs`, add these methods inside `impl App` (after the existing `disable_selected_plugin` method, around line 254):

```rust
pub fn toggle_selected_at_scope(&mut self, scope: Scope) {
    let Some(plugin) = self.selected_plugin() else {
        return;
    };
    let id = plugin.id.clone();
    // Clone the plugin snapshot so we can re-borrow self mutably below.
    let plugin_snapshot = plugin.clone();

    match self.service.toggle_at_scope(&plugin_snapshot, scope) {
        Ok(new_state) => {
            if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
                match scope {
                    Scope::User => p.enabled_user = Some(new_state),
                    Scope::Project => p.enabled_project = Some(new_state),
                    Scope::Local => p.enabled_local = Some(new_state),
                }
            }
            self.message = Some(StatusMessage::info(format!(
                "{} {} in {} scope",
                id,
                if new_state { "enabled" } else { "disabled" },
                scope
            )));
        }
        Err(e) => {
            self.message = Some(StatusMessage::error(format!("Failed to toggle: {}", e)));
        }
    }
}

pub fn enable_selected_at_scope(&mut self, scope: Scope) {
    let Some(plugin) = self.selected_plugin() else {
        return;
    };
    let id = plugin.id.clone();

    match self.service.enable_plugin(&id, scope) {
        Ok(()) => {
            if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
                match scope {
                    Scope::User => p.enabled_user = Some(true),
                    Scope::Project => p.enabled_project = Some(true),
                    Scope::Local => p.enabled_local = Some(true),
                }
            }
            self.message = Some(StatusMessage::info(format!(
                "Enabled {} in {} scope",
                id, scope
            )));
        }
        Err(e) => {
            self.message = Some(StatusMessage::error(format!("Failed to enable: {}", e)));
        }
    }
}

pub fn disable_selected_at_scope(&mut self, scope: Scope) {
    let Some(plugin) = self.selected_plugin() else {
        return;
    };
    let id = plugin.id.clone();

    match self.service.disable_plugin(&id, scope) {
        Ok(()) => {
            if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
                match scope {
                    Scope::User => p.enabled_user = Some(false),
                    Scope::Project => p.enabled_project = Some(false),
                    Scope::Local => p.enabled_local = Some(false),
                }
            }
            self.message = Some(StatusMessage::info(format!(
                "Disabled {} in {} scope",
                id, scope
            )));
        }
        Err(e) => {
            self.message = Some(StatusMessage::error(format!("Failed to disable: {}", e)));
        }
    }
}
```

`Plugin` already derives `Clone` (`src/plugin/mod.rs:84`), so the `clone()` call is free of new derives.

- [ ] **Step 2: Verify the build**

```bash
cargo build
```

Expected: clean build. The new methods add to `App`'s API; existing methods are still present.

- [ ] **Step 3: Lints + test suite**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "Add scope-aware enable/disable/toggle methods to App

Mirrors the existing scope-agnostic helpers but takes an explicit Scope
argument. Used by the new keybindings in main.rs (next task). The old
install_scope-targeting helpers are retained until main.rs no longer
references them; they will be removed in the keybinding task."
```

---

## Task 4: Wire new keybindings in `main.rs`

**Why:** With the discovery fix and action primitives in place, give the user the actual keystrokes: `l`/`Space`/`Enter` toggle Local, `p` toggles Project, `u` toggles User, `e`/`d` enable/disable in Local. Detail modal moves from `Enter` to `i`.

**Files:**
- Modify: `src/main.rs:80-118` (`handle_normal_mode` function) and `src/main.rs:144-153` (`handle_detail_modal_mode`)
- Modify: `src/app.rs` — remove the now-unused old methods (`toggle_selected_plugin`, `enable_selected_plugin`, `disable_selected_plugin`)

- [ ] **Step 1: Update `handle_normal_mode` in `src/main.rs`**

Replace the existing `handle_normal_mode` body (lines ~80-118) with:

```rust
fn handle_normal_mode(app: &mut App, key: KeyCode) {
    use ccpm::plugin::Scope;

    match key {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
        KeyCode::Char('g') => app.select_first(),
        KeyCode::Char('G') => app.select_last(),

        // Default per-project actions (target Local scope)
        KeyCode::Char('e') => app.enable_selected_at_scope(Scope::Local),
        KeyCode::Char('d') => app.disable_selected_at_scope(Scope::Local),
        KeyCode::Char(' ') | KeyCode::Char('l') | KeyCode::Enter => {
            app.toggle_selected_at_scope(Scope::Local)
        }

        // Explicit scope toggles
        KeyCode::Char('p') => app.toggle_selected_at_scope(Scope::Project),
        KeyCode::Char('u') => app.toggle_selected_at_scope(Scope::User),

        // Detail modal moved from Enter to 'i' (info)
        KeyCode::Char('i') => app.show_detail_modal(),

        KeyCode::Char('x') => app.confirm_remove(),

        // Filtering
        KeyCode::Char('s') => app.cycle_scope_filter(),
        KeyCode::Char('/') => app.start_search(),

        // Reload
        KeyCode::Char('r') => {
            if let Err(e) = app.reload_plugins() {
                app.message = Some(ccpm::app::StatusMessage::error(format!(
                    "Reload failed: {}",
                    e
                )));
            } else {
                app.message = Some(ccpm::app::StatusMessage::info("Plugins reloaded"));
            }
        }

        // Help and quit
        KeyCode::Char('?') => app.show_help(),
        KeyCode::Char('q') => app.quit(),
        KeyCode::Esc => app.clear_search(),

        _ => {}
    }
}
```

- [ ] **Step 2: Update `handle_detail_modal_mode` in `src/main.rs`**

Replace lines ~144-153 with:

```rust
fn handle_detail_modal_mode(app: &mut App, key: KeyCode) {
    use ccpm::plugin::Scope;

    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('i') => app.hide_detail_modal(),
        // Toggle from inside the modal — same defaults as Normal mode (Local scope)
        KeyCode::Char(' ') | KeyCode::Char('l') | KeyCode::Enter => {
            app.toggle_selected_at_scope(Scope::Local)
        }
        KeyCode::Char('p') => app.toggle_selected_at_scope(Scope::Project),
        KeyCode::Char('u') => app.toggle_selected_at_scope(Scope::User),
        KeyCode::Char('e') => app.enable_selected_at_scope(Scope::Local),
        KeyCode::Char('d') => app.disable_selected_at_scope(Scope::Local),
        _ => {}
    }
}
```

- [ ] **Step 3: Remove unused App methods**

In `src/app.rs`, delete the now-unreferenced methods:
- `toggle_selected_plugin` (lines ~170-197)
- `enable_selected_plugin` (lines ~199-225)
- `disable_selected_plugin` (lines ~227-253)

These are replaced by the scope-aware versions added in Task 3.

- [ ] **Step 4: Update the footer hint strings in `src/ui/mod.rs`**

In `render_footer` (lines ~127-142), update the `AppMode::Normal` and `AppMode::DetailModal` arm to reflect the new keys:

```rust
AppMode::Normal => vec![
    ("j/k", "navigate"),
    ("Enter/l", "toggle (local)"),
    ("p/u", "toggle (project/user)"),
    ("e/d", "enable/disable"),
    ("i", "details"),
    ("s", "scope filter"),
    ("/", "search"),
    ("?", "help"),
    ("q", "quit"),
],
```

```rust
AppMode::DetailModal => vec![
    ("Esc/i", "close"),
    ("Enter/l", "toggle (local)"),
    ("p/u", "project/user"),
],
```

- [ ] **Step 5: Build and run the test suite**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
```

Expected: green. Existing `app.rs` tests are zero (verified in Task 3 note); the test suite covers `plugin::*` and integration.

- [ ] **Step 6: Smoke test the TUI manually**

```bash
cargo run -- list      # CLI still works
cargo run               # launches TUI
```

Inside the TUI, in the project root:
1. Navigate (`j`/`k`).
2. Press `i` — detail modal opens. Press `i` or `Esc` — closes.
3. Press `l` (or Space, or Enter) on a user-scope plugin that is currently enabled — observe `[+]` flips to `[-]`. Verify `cat .claude/settings.local.json` contains `"<plugin-id>": false`.
4. Press `l` again — verify `[+]` returns and the file flips to `true`.
5. Press `u` — verify the plugin's setting in `~/.claude/settings.json` flipped (use a throwaway plugin or back up first).
6. Press `q` — quits cleanly.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs src/app.rs src/ui/mod.rs
git commit -m "Wire per-scope plugin toggle keybindings (Approach 1)

Default per-project keys (Enter, Space, l, e, d) now write to the Local
scope of the current working directory. Explicit scope keys u and p
write to User and Project respectively. Detail modal moves from Enter
to i (info).

The old install_scope-targeting App methods are removed; main.rs uses
the scope-aware variants added in the previous commit."
```

---

## Task 5: Override marker on plugin list rows

**Why:** With Local overrides invisible from the list, users can't scan for "what did I override here." Add a `↓` marker on rows where any non-install-scope setting is set.

**Files:**
- Modify: `src/plugin/mod.rs` (add a `has_override()` method to `Plugin`)
- Modify: `src/ui/plugin_list.rs:18-28` (render the marker)
- Test: `src/plugin/mod.rs` (existing `mod tests`)

- [ ] **Step 1: Write the failing test**

Append to `mod tests` in `src/plugin/mod.rs`:

```rust
#[test]
fn test_has_override_user_install() {
    let mut plugin = make_test_plugin();
    plugin.install_scope = Scope::User;

    // No settings anywhere → no override
    assert!(!plugin.has_override());

    // Only enabled_user (the install scope) → no override
    plugin.enabled_user = Some(true);
    assert!(!plugin.has_override());

    // Add Project setting → override
    plugin.enabled_project = Some(false);
    assert!(plugin.has_override());

    // Reset, add Local setting → override
    plugin.enabled_project = None;
    plugin.enabled_local = Some(true);
    assert!(plugin.has_override());
}

#[test]
fn test_has_override_local_install() {
    let mut plugin = make_test_plugin();
    plugin.install_scope = Scope::Local;
    plugin.enabled_local = Some(true);

    // Only the install scope is set → no override
    assert!(!plugin.has_override());

    // User setting is an override for a Local-installed plugin
    plugin.enabled_user = Some(true);
    assert!(plugin.has_override());
}
```

- [ ] **Step 2: Run tests to verify failure**

```bash
cargo test --lib plugin::tests::test_has_override
```

Expected: 2 tests fail with "no method named `has_override`".

- [ ] **Step 3: Implement `has_override`**

In `src/plugin/mod.rs`, add this method to `impl Plugin` (near `scope_indicator`, around line 166):

```rust
/// Returns true when any non-install-scope `enabled_*` field is set.
/// Used to render the "↓" override marker in the plugin list.
pub fn has_override(&self) -> bool {
    match self.install_scope {
        Scope::User => self.enabled_project.is_some() || self.enabled_local.is_some(),
        Scope::Project => self.enabled_user.is_some() || self.enabled_local.is_some(),
        Scope::Local => self.enabled_user.is_some() || self.enabled_project.is_some(),
    }
}
```

- [ ] **Step 4: Run tests to verify pass**

```bash
cargo test --lib plugin::tests
```

Expected: all green.

- [ ] **Step 5: Render the marker in `src/ui/plugin_list.rs`**

In `render_plugin_list` (lines ~12-56), insert an override-marker `Span` after the existing `scope_indicator` span. Replace the `ListItem::new(Line::from(vec![ ... ]))` block (lines ~49-54) with:

```rust
let override_marker = if plugin.has_override() {
    Span::styled("↓", Style::default().fg(Color::Yellow))
} else {
    Span::raw(" ")
};

ListItem::new(Line::from(vec![
    scope_indicator,
    override_marker,
    status_indicator,
    name,
    marketplace,
]))
```

The single-character marker keeps row width predictable (always 1 column whether or not the override is present).

- [ ] **Step 6: Run lints + visual check**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
cargo run     # launch TUI; verify ↓ appears next to overridden rows
```

- [ ] **Step 7: Commit**

```bash
git add src/plugin/mod.rs src/ui/plugin_list.rs
git commit -m "Surface override marker in plugin list rows

Each plugin gets a Plugin::has_override() helper that returns true when
any non-install-scope enabled_* field is Some(_). The list row renders
↓ in yellow when has_override() is true, blank otherwise. Lets users
scan the list and spot 'this plugin's enabled state has been customized
in another scope' at a glance."
```

---

## Task 6: Settings block in details pane

**Why:** When a row's `[+]/[-]` doesn't match user expectation, the details pane should explain *why* — show the per-scope settings and which one is winning.

**Files:**
- Modify: `src/ui/details.rs` (add a Settings block after the existing "Enabled in" line)

- [ ] **Step 1: Add format helper at the top of `src/ui/details.rs`**

Below the `use` statements, add:

```rust
fn format_setting(value: Option<bool>) -> Span<'static> {
    match value {
        Some(true) => Span::styled("enabled", Style::default().fg(Color::Green)),
        Some(false) => Span::styled("disabled", Style::default().fg(Color::Red)),
        None => Span::styled("(no setting)", Style::default().fg(Color::DarkGray)),
    }
}
```

- [ ] **Step 2: Insert Settings block in `render_details`**

In `render_details`, after the existing "Enabled in" line block (lines ~48-54) and before the project-path block (line ~57), insert:

```rust
        // Per-scope settings breakdown
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Settings:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::raw("  User:    "),
            format_setting(plugin.enabled_user),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Project: "),
            format_setting(plugin.enabled_project),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Local:   "),
            format_setting(plugin.enabled_local),
        ]));

        let effective_label = match plugin.effective_scope() {
            Some(scope) => format!("Effective: {} ({})",
                if plugin.is_enabled() { "ENABLED" } else { "DISABLED" },
                scope),
            None => "Effective: DISABLED (no settings)".to_string(),
        };
        lines.push(Line::from(Span::styled(
            effective_label,
            Style::default()
                .fg(if plugin.is_enabled() { Color::Green } else { Color::Red })
                .add_modifier(Modifier::BOLD),
        )));
```

- [ ] **Step 3: Build and verify visually**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
cargo run      # launch TUI; check the right-hand details pane on a plugin with overrides
```

Expected: pane shows User/Project/Local breakdown plus an "Effective: ENABLED/DISABLED (Scope)" line.

- [ ] **Step 4: Commit**

```bash
git add src/ui/details.rs
git commit -m "Show per-scope settings breakdown in details pane

Adds a Settings block that prints User / Project / Local each as
'enabled', 'disabled', or '(no setting)', followed by an Effective line
indicating which scope's setting is winning. Makes the precedence
visible — answers 'why is this plugin's row showing [-] when User says
true?' at a glance."
```

---

## Task 7: Override count in status bar

**Why:** Spec section 4.4 — `[overrides: N]` in the header gives a project-wide hygiene number.

**Files:**
- Modify: `src/app.rs` (add `override_count` helper)
- Modify: `src/ui/mod.rs:62-124` (`render_header`)
- Test: `src/plugin/mod.rs` already covers `has_override`; the count is a one-liner that doesn't merit its own unit test.

- [ ] **Step 1: Add `override_count` to `App`**

In `src/app.rs`, add to `impl App` (near `plugin_count`, around line 303):

```rust
/// Count of plugins in the current filtered view that have a non-install-scope override.
pub fn override_count(&self) -> usize {
    self.filtered_plugins
        .iter()
        .filter_map(|&i| self.plugins.get(i))
        .filter(|p| p.has_override())
        .count()
}
```

- [ ] **Step 2: Render the count in the header**

In `src/ui/mod.rs`, inside `render_header` (around line 99), append a new span to the `title` vector after the "{enabled}/{total} enabled" span:

```rust
        Span::raw("│ "),
        Span::styled(
            format!("[overrides: {}] ", app.override_count()),
            Style::default().fg(Color::Yellow),
        ),
```

- [ ] **Step 3: Verify**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
cargo run     # launch TUI; verify "[overrides: N]" is in the header
```

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/ui/mod.rs
git commit -m "Add override count to TUI header

Header now shows '[overrides: N]' for the count of plugins in the
currently filtered view that have any non-install-scope setting. Cheap
derivation; updates whenever the filter or any toggle changes."
```

---

## Task 8: Update help overlay

**Why:** The help screen still documents the pre-feature keybindings (`Space` toggles, `Enter` views details, `u` toggles auto-update which never existed in main.rs anyway). Bring it in line.

**Files:**
- Modify: `src/ui/help.rs:16-50` (the `keybindings` vec)

- [ ] **Step 1: Replace the keybindings vec**

In `src/ui/help.rs`, replace the `keybindings` vec (lines ~16-50) with:

```rust
    let keybindings = vec![
        (
            "Navigation",
            vec![
                ("j / ↓", "Move down"),
                ("k / ↑", "Move up"),
                ("g", "Go to first"),
                ("G", "Go to last"),
                ("i", "View detail modal"),
            ],
        ),
        (
            "Plugin Actions (default = Local scope of CWD)",
            vec![
                ("Enter / l / Space", "Toggle in Local scope"),
                ("e", "Enable in Local scope"),
                ("d", "Disable in Local scope"),
                ("p", "Toggle in Project scope (committed)"),
                ("u", "Toggle in User scope (global)"),
                ("x", "Remove plugin (placeholder)"),
            ],
        ),
        (
            "Filtering",
            vec![
                ("s", "Cycle scope filter (All/User/Project/Local)"),
                ("/", "Start search"),
                ("Esc", "Clear search / Exit mode"),
            ],
        ),
        (
            "General",
            vec![
                ("?", "Toggle help"),
                ("r", "Reload plugins"),
                ("q", "Quit"),
            ],
        ),
    ];
```

- [ ] **Step 2: Add a footer note about project root**

After the keybindings rendering loop, before the existing "Press ? or Esc to close" line (line ~85), insert:

```rust
    lines.push(Line::from(Span::styled(
        "Tip: run from a project root (where .claude/ lives), not $HOME.",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
```

- [ ] **Step 3: Verify**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
cargo run    # press ? to verify the help overlay
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/help.rs
git commit -m "Update help overlay for per-scope toggles

Documents Approach 1 keybindings: Enter/l/Space write Local, p writes
Project, u writes User; e/d enable/disable in Local; i opens the detail
modal (moved from Enter). Adds a tip about running from project root
to avoid creating .claude/ in subdirectories or \$HOME."
```

---

## Task 9: Integration test — gitlab-style override scenario

**Why:** The unit tests prove the layers. The integration test proves the user-facing CLI surfaces the effective state correctly when a Local override is in play. Mirrors the use case from the spec ("gitlab globally enabled, github project disables it").

**Files:**
- Modify: `tests/integration.rs` (append a new test)

**Note:** Existing integration tests run against the real user environment via `cargo_bin("ccpm")`. The new test needs an isolated `HOME` and `CWD`. Use the `tempfile::TempDir` + `Command::env` pattern. CCPM uses `dirs::home_dir()` (driven by `HOME` on Unix, `USERPROFILE` on Windows), and reads CWD via `env::current_dir()` for project settings — `Command::current_dir` overrides the latter.

- [ ] **Step 1: Write the failing test**

Append to `tests/integration.rs`:

```rust
use std::fs;
use tempfile::TempDir;

#[test]
fn test_user_scope_plugin_with_local_override_is_disabled_in_cwd() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    // Set up the fake user home: install + enable gitlab globally
    let claude_dir = home.path().join(".claude");
    let plugins_dir = claude_dir.join("plugins");
    fs::create_dir_all(&plugins_dir).unwrap();

    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"gitlab@market":true}}"#,
    )
    .unwrap();

    fs::write(
        plugins_dir.join("installed_plugins.json"),
        r#"{
            "version": 2,
            "plugins": {
                "gitlab@market": [{
                    "scope": "user",
                    "installPath": "/fake/path",
                    "version": "1.0.0",
                    "installedAt": "2026-01-01T00:00:00Z",
                    "lastUpdated": "2026-01-01T00:00:00Z"
                }]
            }
        }"#,
    )
    .unwrap();

    // Project local override: disabled
    let project_claude = project.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.local.json"),
        r#"{"enabledPlugins":{"gitlab@market":false}}"#,
    )
    .unwrap();

    // Run `ccpm list --debug` in project; assert effective state
    let output = Command::cargo_bin("ccpm")
        .unwrap()
        .args(["list", "--debug"])
        .env("HOME", home.path())
        .current_dir(project.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "ccpm list failed: {}", stderr);
    assert!(
        stderr.contains("enabled_local=Some(false)") || stderr.contains("local=Some(false)"),
        "debug output should show local override; got:\n{}",
        stderr
    );
    assert!(
        stdout.contains("disabled"),
        "gitlab should appear disabled in list output; got:\n{}",
        stdout
    );
}

#[test]
fn test_cli_disable_with_scope_local_writes_local_settings() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    // Minimal fake user home so ccpm can run
    fs::create_dir_all(home.path().join(".claude/plugins")).unwrap();
    fs::write(
        home.path().join(".claude/plugins/installed_plugins.json"),
        r#"{"version":2,"plugins":{}}"#,
    )
    .unwrap();

    let status = Command::cargo_bin("ccpm")
        .unwrap()
        .args(["disable", "demo@market", "--scope", "local"])
        .env("HOME", home.path())
        .current_dir(project.path())
        .status()
        .unwrap();
    assert!(status.success(), "disable command failed");

    let local_settings = project.path().join(".claude/settings.local.json");
    assert!(local_settings.exists(), "settings.local.json not created");

    let content = fs::read_to_string(&local_settings).unwrap();
    assert!(
        content.contains("\"demo@market\""),
        "settings.local.json missing entry: {}",
        content
    );
    assert!(
        content.contains("false"),
        "settings.local.json should set demo@market to false: {}",
        content
    );
}
```

- [ ] **Step 2: Run tests to verify the override-fixture test passes against the discovery fix**

```bash
cargo test --test integration test_user_scope_plugin_with_local_override
cargo test --test integration test_cli_disable_with_scope_local
```

Expected: both pass. If `enabled_local=Some(false)` substring isn't in the debug format, adjust the assertion to match the existing `eprintln!` format in `src/cli/mod.rs:115` — check that file before writing the assertion. The current format string is `local={:?}` which renders as `local=Some(false)`.

- [ ] **Step 3: Run full suite**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "Add integration tests for per-project plugin overrides

Two scenarios covered end-to-end via the ccpm binary:
1. A user-scope plugin (gitlab) enabled in HOME but disabled in CWD's
   settings.local.json shows up as disabled in 'ccpm list', proving the
   discovery fix wires through to the CLI surface.
2. 'ccpm disable <id> --scope local' creates and populates the project's
   settings.local.json correctly when run from a clean directory."
```

---

## Task 10: Documentation updates

**Why:** `CLAUDE.md` rules require docs to be updated after every significant change. This is a user-visible feature (keybindings changed) and an architectural one (discovery semantics changed).

**Files:**
- Modify: `CLAUDE.md` — keybindings reference, in-progress section
- Modify: `docs/architecture.md` — Plugin struct, settings loading strategy
- Modify: `FEATURE_PLAN.md` — move B and #6 to Completed
- Modify: `README.md` — keybindings table, only the user-visible changes
- Modify: `CLAUDE.local.md` — implementation notes section
- Delete: `.claude/ccpm-task-scope.md` if it exists (temporal artifact for the now-replaced Modal-mode plan)

- [ ] **Step 1: Check for the old task file**

```bash
ls -la .claude/ccpm-task-scope.md 2>&1
```

If it exists, read it first (`cat .claude/ccpm-task-scope.md`) to confirm its content is fully captured by the new spec/plan. If yes, plan to delete in Step 6.

- [ ] **Step 2: Update `CLAUDE.md`**

Find the "## CLI Commands" section. Add after the existing TUI subsection (or in a new "## TUI Keybindings" subsection):

```markdown
## TUI Keybindings

In Normal mode (the default):

| Key | Action |
|-----|--------|
| `Enter` / `l` / `Space` | Toggle plugin in **Local** scope (`./.claude/settings.local.json`) |
| `e` | Enable plugin in Local scope |
| `d` | Disable plugin in Local scope |
| `p` | Toggle plugin in Project scope (`./.claude/settings.json`, committed) |
| `u` | Toggle plugin in User scope (`~/.claude/settings.json`, global) |
| `i` | Show detail modal |
| `s` | Cycle scope filter |
| `/` | Search |
| `?` | Help |
| `r` | Reload plugins |
| `q` | Quit |

The default actions (`Enter` / `l` / `Space` / `e` / `d`) write to **Local** scope of the current working directory, matching the per-project workflow. Use `u` and `p` for explicit User or Project writes.

**Tip:** run CCPM from a project root (where `.claude/` lives) — running from a subdirectory will create `.claude/` at the wrong level.
```

Also find the "### Plugin Discovery (`plugin/discovery.rs`)" subsection and update the second bullet:

```markdown
- For all plugins (including user-scope), `enabled_project` and `enabled_local` are populated from the current working directory's `.claude/settings.json` and `.claude/settings.local.json` so the effective Local > Project > User precedence is reflected in the UI.
- For project/local scope plugins: settings come from the plugin's own `projectPath` (cross-project isolation preserved).
```

Remove the "### Scope Selection (Feature B)" entry from "## In-Progress Features" (the feature is now complete; details live in `docs/superpowers/specs/2026-05-04-per-project-plugin-scoping-design.md`).

- [ ] **Step 3: Update `docs/architecture.md`**

Find the "### Settings Loading Strategy" section (around the bottom). Update the prose paragraph that begins "// For each plugin:" — replace with:

```markdown
For every plugin (regardless of `install_scope`), CCPM populates `enabled_project` and `enabled_local` from the current working directory's `.claude/settings.json` and `.claude/settings.local.json`. For `Scope::Project` and `Scope::Local` installs, an additional read happens from the plugin's own `project_path` (so a plugin installed in `~/Projects/foo` continues to reflect `foo`'s settings, not the CWD's). User-scope plugins read CWD only — they have no `project_path`. The `is_enabled()` precedence (Local > Project > User) then determines the effective state.
```

Also update the prose at the top of the file: where it says "Three-Scope System", change references to "User scope" and "Local scope" to consistently include "Project scope" — the file currently flags itself as outdated in `CLAUDE.local.md`. Bring the Plugin struct documentation in sync with `src/plugin/mod.rs:84-107` if it has drifted.

- [ ] **Step 4: Update `FEATURE_PLAN.md`**

Move "B. Scope Selection on Enable/Disable" from "## In Progress Features" to "## Completed Features" with a new entry:

```markdown
### B. Per-Project Plugin Scoping (2026-05-04)

Implements Approach 1 keybindings: `Enter` / `l` / `Space` toggle the Local scope of the current working directory; `p` toggles Project; `u` toggles User; `e` / `d` enable/disable in Local. Detail modal moved from `Enter` to `i`. Combined with the discovery fix that lets user-scope plugins respect CWD overrides (formerly item #6).

Spec: `docs/superpowers/specs/2026-05-04-per-project-plugin-scoping-design.md`.

Files modified: `src/plugin/discovery.rs`, `src/plugin/operations.rs`, `src/plugin/mod.rs`, `src/app.rs`, `src/main.rs`, `src/ui/plugin_list.rs`, `src/ui/details.rs`, `src/ui/help.rs`, `src/ui/mod.rs`, `tests/integration.rs`.
```

Move "### 6. Local enabledPlugins Override for User-Scope Plugins" from "## Planned Features" to "## Completed Features" and link it to the B entry above (or merge into it — same PR).

- [ ] **Step 5: Update `README.md`**

Find the keybindings or "Usage" section. Replace whatever toggle-related text is there with a brief mirror of the Step 2 keybindings table. Keep it concise — README is the user-facing surface; full reference lives in `CLAUDE.md` and the in-app help.

- [ ] **Step 6: Update `CLAUDE.local.md`**

Append a new section under "Low-Level Implementation Details":

```markdown
### Per-Project Scoping Feature (COMPLETED 2026-05-04)

**Files modified:**
- `src/plugin/discovery.rs` — `Scope::User` arm now populates `enabled_project` / `enabled_local` from CWD.
- `src/plugin/operations.rs` — new `PluginService::toggle_at_scope(plugin, scope)`.
- `src/plugin/mod.rs` — new `Plugin::has_override()` helper.
- `src/app.rs` — replaces `toggle_selected_plugin` / `enable_selected_plugin` / `disable_selected_plugin` with `*_at_scope(scope)` variants. New `App::override_count()`.
- `src/main.rs` — `Enter` / `l` / `Space` / `e` / `d` write Local; `p` writes Project; `u` writes User; `i` opens detail modal.
- `src/ui/plugin_list.rs` — `↓` override marker.
- `src/ui/details.rs` — Settings block (per-scope breakdown + Effective line).
- `src/ui/mod.rs` — `[overrides: N]` in header, footer hints updated.
- `src/ui/help.rs` — keybindings reference.

**Key implementation details:**
- The `toggle_at_scope` "first press flips effective state" rule lives at the `PluginService` layer (not `App`) so it's testable via `with_paths`.
- The override marker uses a single yellow `↓` glyph; rows without an override use a plain space so column widths stay constant.
- `Plugin::has_override()` checks any non-install-scope `enabled_*` field, not just Local.
```

Also update or remove the "Active Task Files" section if `.claude/ccpm-task-scope.md` is being deleted in Step 7.

- [ ] **Step 7: Delete the old task file (if it exists)**

```bash
ls -la .claude/ccpm-task-scope.md 2>&1
# If present and content fully captured in the spec/plan above:
rm -f .claude/ccpm-task-scope.md
```

- [ ] **Step 8: Final full verification**

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
```

Expected: green.

- [ ] **Step 9: Commit**

```bash
git add CLAUDE.md docs/architecture.md FEATURE_PLAN.md README.md CLAUDE.local.md
git rm -f .claude/ccpm-task-scope.md 2>/dev/null || true
git commit -m "Update docs for per-project plugin scoping feature

CLAUDE.md: add TUI keybindings reference; refresh discovery description.
docs/architecture.md: update Settings Loading Strategy and Plugin struct.
FEATURE_PLAN.md: move Feature B and item #6 to Completed.
README.md: refresh keybindings table.
CLAUDE.local.md: add implementation notes.
Removes the now-stale .claude/ccpm-task-scope.md task file."
```

---

## Final verification

After Task 10 commits, run:

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check
git log --oneline -10
git status
```

Expected:
- All tests pass.
- No clippy warnings.
- Formatting clean.
- 10 new commits on top of `60e4aca` (the spec commit), one per task.
- Clean working tree.

Then sanity-check the user-facing flow:

```bash
cargo run                      # opens TUI in this repo's directory (~/Projects/ccpm)
# - press 'l' on a user-scope plugin
# - cat .claude/settings.local.json — verify entry written
# - press 'l' again — verify entry flipped
# - press 'i' — detail modal opens with Settings block
# - press 'i' or 'Esc' — closes
# - quit with 'q'
```

If everything looks right, the feature is shippable.

---

## What this plan does NOT do (and shouldn't)

These are deferred per the spec. Do not pull them in:

- **Walk-up project root discovery** from CWD (Scope 1)
- **`--project <path>` flag** (Scope 1)
- **`~/.config/ccpm/config.toml`** with `projects_root` (ships with multi-project view)
- **Multi-project dashboard scanning `~/Projects/*`** (feature C, backlog)
- **Profiles / presets** (feature D, backlog)
- **Hygiene features** (bulk select, decision-support columns) (feature A, deprioritized)
- **`Shift+l` / `d` to remove an override** (3-state cycle)
- **Modal scope-picker dialog** (Approach 2; the existing const + enum + AppMode::ScopeSelect were design notes that never landed in source — do not implement them now)
- **`~/.claude/plugins/cache/` invalidation** (undocumented; only revisit if testing surfaces a stale-state bug)

If any of these become tempting during implementation, stop and ask first.
