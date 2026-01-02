# CCPM Feature Plan

This document tracks planned features and ideas for future development.

## In Progress Features

### B. Scope Selection on Enable/Disable

**Status**: Planned (2026-01-02)
**Priority**: High
**Context File**: `.claude/ccpm-task-scope.md`

Current problem: Toggle uses `install_scope` which may not match user preference. Creates `settings.json` even if user only uses `settings.local.json`.

Implementation:
- `ScopeSelectionMode` enum: Modal, Inline, Keybinding (compile-time const)
- Keybindings: `u`/`p`/`l` for direct scope selection
- Enter key triggers scope selection dialog
- `AppMode::ScopeSelect` for dialog state

---

### C. Persist Default Scope Selection

**Status**: FUTURE (do not implement yet)
**Priority**: Low
**Depends On**: Feature B (Scope Selection)

Store user's default scope preference in `.claude/ccpm.local.json`:

```json
{
  "defaultScope": "local",
  "showScopePrompt": false
}
```

On first run (or when no default set):
- Show prompt: `Select default scope: (U)ser  (P)roject  (L)ocal`
- Capital letter indicates current default
- Save choice for future sessions

---

## Planned Features

### 1. Development Plugin Support

**Status**: Not implemented
**Priority**: Low
**Rationale**: Development plugins loaded via `--plugin-dir` are not tracked in `installed_plugins.json`. They are ephemeral and meant for plugin developers testing their work.

#### Option A: Ignore (Current)
Do nothing. These plugins are for developers, not end-users managing installed plugins.

#### Option B: Detect Running Plugins
Parse running Claude processes or check runtime config to discover `--plugin-dir` loaded plugins. Complex and fragile.

#### Option C: Manual Dev Plugin Registry (Recommended Future Feature)
Add a command or TUI action to manually register development plugin paths:

```bash
# CLI approach
ccpm dev-plugin add /path/to/my-plugin
ccpm dev-plugin remove /path/to/my-plugin
ccpm dev-plugin list

# Or TUI approach
# Press 'd' to open dev plugin management
# Add/remove paths to development plugins
```

Store in `~/.config/ccpm/dev_plugins.json`:
```json
{
  "devPlugins": [
    {
      "path": "/Users/me/projects/my-plugin",
      "addedAt": "2025-01-01T00:00:00Z",
      "name": "my-plugin"  // Read from plugin.json
    }
  ]
}
```

Display in TUI with special indicator: `[D]` for dev plugins.

**Why not now**: Scope creep. Focus on core three-scope feature first.

---

### 2. enabledPlugins Visibility

**Status**: Not implemented
**Priority**: Medium

Show plugins configured in `enabledPlugins` but not yet installed. Would help users understand what plugins a project expects.

```
enabledPlugins in .claude/settings.json:
  - foo@marketplace (installed)
  - bar@marketplace (not installed - pending)
```

---

### 3. Marketplace Browser

**Status**: Not implemented
**Priority**: Medium

Browse available plugins from configured marketplaces within CCPM TUI.

---

### 4. Plugin Update Checker

**Status**: Not implemented
**Priority**: Low

Compare installed versions against marketplace versions, show update availability.

---

### 5. Install / Delete Plugin

**Status**: Not implemented
**Priority**: Low

Provide interface to control plugin installation and deletion from plugin manager.

---

## Completed Features

### Cross-Project Settings Isolation Fix (2026-01-02)

**Bug**: Plugins installed in Project A were reading their enabled state from the CWD's `.claude/` directory instead of from Project A's settings.

**Example**: Plugin `agent-orchestration` installed in `~/Projects/Ternv3` (Local scope) would show the wrong enabled state when CCPM was run from `~/Projects/ccpm`.

**Root cause**: `PluginDiscovery::discover_all()` loaded project/local settings from CWD for ALL plugins, regardless of where they were installed. This is correct for plugins installed in the current project, but wrong for plugins installed in other projects.

**Fix**:
- Added `ConfigPaths::load_settings_from_project(project_path)` helper to load settings from any project directory
- Modified `discover_all()` to check `entry.project_path` for each plugin
- For project/local scope plugins with a `project_path`, settings are now loaded from that project's `.claude/` directory
- Added settings cache to avoid re-reading the same project's settings multiple times

**CLI Debug Flag**:
- Added `--debug` flag to `ccpm list` command
- Outputs Option values (`enabled_user`, `enabled_project`, `enabled_local`) and `project_path` to stderr
- Helps diagnose settings-related issues

Files modified: `src/plugin/config.rs`, `src/plugin/discovery.rs`, `src/cli/mod.rs`

---

### Corrupted Lock File Handling (2026-01-02)

**Enhancement**: Lock files that contain invalid JSON or are empty are now treated as stale and automatically deleted, allowing new lock acquisition to proceed.

Files modified: `src/plugin/operations.rs`

---

### Settings Precedence Bug Fix (2026-01-02)

**Bug**: Local `false` didn't override Project `true`. CCPM incorrectly showed plugins as enabled when local settings explicitly disabled them.

**Root cause**: `enabled_user/project/local` were `bool` fields where `false` meant both "no setting" and "explicitly disabled". The `is_enabled()` function only checked if a scope was `true`, ignoring explicit `false` settings.

**Fix**:
- Changed enabled fields from `bool` to `Option<bool>`
- `None` = no setting in that scope (fall through to next)
- `Some(true)` = explicitly enabled
- `Some(false)` = explicitly disabled
- Rewrote `is_enabled()` with correct precedence: Local > Project > User

Files modified: `src/plugin/mod.rs`, `src/plugin/discovery.rs`, `src/app.rs`, `README.md`

---

### A. Vim-style Lock File Handling (2026-01-02)

Lock files (`settings.lock`, etc.) are now properly managed:
- `LockFileGuard` struct auto-deletes lock file on Drop (normal completion or panic)
- Lock file contains JSON with PID and timestamp for debugging
- Stale lock detection: checks if holding process is still running
- Returns `LockConflict` error for active locks (TUI can show dialog)
- Cross-platform: Unix uses `kill -0`, non-Unix conservatively assumes active

Files modified: `src/plugin/operations.rs`, `src/plugin/mod.rs`

---

- [x] Basic TUI plugin list
- [x] User/Local scope display (partial - needs Project scope)
- [x] Enable/disable plugins
- [x] Search/filter plugins
- [x] Detail modal
- [x] Vim-style lock file handling (Feature A)
- [x] Settings precedence bug fix (Local > Project > User)

---

## References

- [Claude Code Plugins Docs](https://code.claude.com/docs/en/plugins)
- [Claude Code CLI Reference](https://code.claude.com/docs/en/cli-reference)
- [Project-scope bug: Issue #14202](https://github.com/anthropics/claude-code/issues/14202)
