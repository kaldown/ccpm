# CCPM Feature Plan

This document tracks planned features and ideas for future development.

## In Progress Features

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

### 0. Project Initialization (`ccpm init`)

**Status**: Not implemented
**Priority**: High
**Rationale**: Streamline Claude Code onboarding by bootstrapping project configuration with recommended settings.

#### Problem
Setting up Claude Code for a new project requires manually:
- Creating `.claude/` directory structure
- Copying hooks from other projects or documentation
- Configuring sandbox permissions
- Installing and enabling recommended plugins
- Setting up project-specific configuration

This is time-consuming and error-prone, especially for teams onboarding multiple projects.

#### Proposed Solution
Add `ccpm init` command to bootstrap a project with recommended Claude Code configuration:

```bash
ccpm init .                    # Initialize current directory
ccpm init /path/to/project     # Initialize specific project
ccpm init --template rust      # Use Rust project template
ccpm init --interactive        # Interactive setup wizard
```

#### What Gets Initialized

1. **Directory Structure**
   - `.claude/` directory
   - `.claude/settings.json` (project-scope)
   - `.gitignore` entry for `settings.local.json`

2. **Hooks** (optional, user-selected)
   - Pre-commit hooks (linting, formatting checks)
   - User prompt submit hooks (validation, safety checks)
   - Example hooks from common workflows

3. **Plugins** (optional, user-selected)
   - Language-specific plugins (rust-analyzer-lsp, python-lsp, etc.)
   - Common workflow plugins (feature-dev, debugging-toolkit, etc.)
   - Project-specific recommendations based on detected tech stack

4. **Sandbox Configuration**
   - Filesystem allowlist patterns (workspace, build dirs, temp dirs)
   - Network allowlist (package registries, docs sites)
   - Unix socket permissions (Docker, etc.)

5. **Documentation**
   - `CLAUDE.md` template with project context instructions, including:
     - Essential Claude Code usage hints (e.g., "Use context7 for library documentation")
     - Required plugins for this project type with rationale
     - Project-specific conventions and patterns
     - Build/test commands
     - Architecture overview placeholders
   - `.claude/README.md` explaining the configuration

#### Implementation Options

**Option A: Template-based (Recommended)**
- Ship predefined templates for common project types (Rust, Python, Node.js, etc.)
- Templates stored in `~/.claude/ccpm/templates/` or embedded in binary
- Users can create custom templates in `~/.claude/ccpm/templates/custom/`

**Option B: Interactive Wizard**
- Detect project type from files (Cargo.toml, package.json, requirements.txt)
- Ask questions: "Enable pre-commit hooks? (y/n)"
- Build configuration interactively

**Option C: Hybrid (Best UX)**
- Detect project type automatically
- Offer template as default: "Detected Rust project. Use Rust template? (Y/n)"
- Allow interactive customization: `--interactive` flag
- Allow template selection: `--template <name>` flag

#### Template Structure

```
~/.claude/ccpm/templates/
├── rust/
│   ├── settings.json
│   ├── hooks/
│   │   └── user-prompt-submit.sh
│   ├── plugins.txt          # List of recommended plugin IDs
│   ├── sandbox.json         # Sandbox configuration snippet
│   └── CLAUDE.md            # Template with placeholders
├── python/
│   └── ...
└── node/
    └── ...
```

#### CLI Interface

```bash
# Basic usage
ccpm init .

# Outputs:
# Initializing Claude Code project...
# ✓ Created .claude/ directory
# ✓ Created .claude/settings.json
# ✓ Added .claude/settings.local.json to .gitignore
#
# Install recommended plugins? (y/N): y
# [1] rust-analyzer-lsp - LSP support for Rust
# [2] feature-dev - Guided feature development
# Select plugins (comma-separated numbers or 'all'): 1,2
# ✓ Added plugins to settings.json (run 'claude plugin install' to install)
#
# Set up pre-commit hooks? (y/N): y
# ✓ Created .claude/hooks/user-prompt-submit.sh
#
# Configure sandbox? (y/N): y
# ✓ Added sandbox configuration to settings.json
#
# Initialization complete!
# Run 'claude plugin install' to install enabled plugins.
```

#### Files Modified
- `src/cli/mod.rs` - Add `init` subcommand
- `src/cli/init.rs` - New module for init logic
- `src/plugin/templates.rs` - Template management
- `Cargo.toml` - Possibly add `include_dir` for embedded templates

#### Future Enhancements
- Remote templates: `ccpm init --template github:user/repo`
- Team templates: Share templates via git
- Template validation and linting
- Update existing projects: `ccpm init --update` (merge new recommendations)

---

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

### B. Per-Project Plugin Scoping (2026-05-04)

Implements Approach 1 keybindings: `Enter` / `l` / `Space` toggle the Local scope of the current working directory; `p` toggles Project; `u` toggles User; `e` / `d` enable/disable in Local. Detail modal moved from `Enter` to `i`. Combined with the discovery fix that lets user-scope plugins respect CWD overrides (formerly item #6).

Spec: `docs/superpowers/specs/2026-05-04-per-project-plugin-scoping-design.md`.

Files modified: `src/plugin/discovery.rs`, `src/plugin/operations.rs`, `src/plugin/mod.rs`, `src/app.rs`, `src/main.rs`, `src/ui/plugin_list.rs`, `src/ui/details.rs`, `src/ui/help.rs`, `src/ui/mod.rs`, `tests/integration.rs`.

---

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
- [x] User/Project/Local scope display
- [x] Enable/disable plugins
- [x] Search/filter plugins
- [x] Detail modal
- [x] Vim-style lock file handling (Feature A)
- [x] Settings precedence bug fix (Local > Project > User)
- [x] Per-project plugin scoping with per-scope keybindings (Feature B + item #6)

---

## References

- [Claude Code Plugins Docs](https://code.claude.com/docs/en/plugins)
- [Claude Code CLI Reference](https://code.claude.com/docs/en/cli-reference)
- [Project-scope bug: Issue #14202](https://github.com/anthropics/claude-code/issues/14202)
