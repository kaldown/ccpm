# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Documentation Hierarchy

This project maintains structured documentation. **Read the appropriate files based on context needed.**

### Committed Documentation (part of repo)

| File | Purpose | When to Read | When to Update |
|------|---------|--------------|----------------|
| `CLAUDE.md` | Primary Claude context, high-to-mid level | Auto-read by Claude Code | After significant changes |
| `docs/architecture.md` | Detailed architecture, data models, component hierarchy | When making architectural changes or need deep context | After structural/architectural changes |
| `FEATURE_PLAN.md` | Feature tracking, planned/in-progress/completed | When planning work or checking status | After completing features |
| `README.md` | User-facing documentation | When updating user-facing features | Only for significant user-visible changes |

### Local Documentation (gitignored)

| File | Purpose | When to Read |
|------|---------|--------------|
| `CLAUDE.local.md` | Low-level details, links, templates, preferences | When need ralph-loop setup, external links, implementation details |
| `.claude/ralph-wiggum-docs.md` | Ralph-wiggum plugin reference | When running ralph-loops |
| `.claude/ccpm-task-*.md` | Temporal task files | During active task execution |

### Reading Order for Full Context

1. `CLAUDE.md` (this file) - Always read first (auto-read)
2. `docs/architecture.md` - For architectural understanding
3. `FEATURE_PLAN.md` - For current work status
4. `CLAUDE.local.md` - For low-level details and templates

## Documentation Update Rules

**After every significant change:**

1. **Architectural changes** (new modules, data structures, component changes):
   - Update `docs/architecture.md` with new/modified structures
   - Update `CLAUDE.md` if it affects high-level understanding

2. **Feature completion**:
   - Update `FEATURE_PLAN.md` - move to completed section
   - Update `CLAUDE.md` - remove from in-progress, add to completed if relevant
   - Update `CLAUDE.local.md` - add implementation details
   - Update `docs/architecture.md` - if new components/patterns added

3. **User-facing changes**:
   - Update `README.md` - only significant user-visible changes
   - Do NOT update for internal changes (lock files, scope selection UI)

4. **Before next execution phase**:
   - Verify all docs are current
   - Clean up temporal artifacts (task files)
   - Ensure no stale information remains

## Artifact Cleanup Policy

**Temporal artifacts** (task files, phase-specific context files) MUST be cleaned up after task completion.

### What are temporal artifacts?
- `.claude/ccpm-task-*.md` - Ralph-loop task files (deleted after task completion)
- Phase-specific context files that duplicate info now in permanent docs
- Any file created for a specific task that won't be needed for future context

### Cleanup behavior:
1. After completing a task, check for redundant/temporal files
2. Verify the file's content is captured in permanent docs (CLAUDE.md, CLAUDE.local.md, FEATURE_PLAN.md)
3. **Ask the user** to confirm deletion: "Task complete. The following temporal files can be removed: [list]. Confirm?"
4. Delete confirmed files
5. Do NOT delete files that may be needed for future phases or contain unique context

### Permanent files (never auto-delete):
- `CLAUDE.md`, `CLAUDE.local.md` - Project context
- `FEATURE_PLAN.md` - Feature tracking
- `README.md` - User documentation
- Source code and tests

## Build & Development Commands

```bash
cargo build              # Build debug
cargo build --release    # Build release (LTO enabled)
cargo test               # Run all tests
cargo test --lib         # Run unit tests only
cargo test integration   # Run integration tests only
cargo clippy -- -D warnings  # Lint (CI enforces zero warnings)
cargo fmt --check        # Check formatting
cargo check              # Fast type-check
```

Run a specific test:
```bash
cargo test test_plugin_is_enabled
```

Install locally:
```bash
cargo install --path .
```

## Architecture

CCPM is a TUI application for managing Claude Code plugins. It reads/writes Claude Code's configuration files to enable/disable plugins.

### Module Structure

```
src/
├── main.rs          # Entry point, terminal setup, event loop, key handlers
├── lib.rs           # Public exports (App, Plugin, PluginService, Scope)
├── app.rs           # App state machine (modes: Normal, Search, Help, Confirm, DetailModal)
├── cli/mod.rs       # CLI subcommands (list, enable, disable, info)
├── plugin/
│   ├── mod.rs       # Plugin struct, Scope enum, ScopeFilter, PluginError
│   ├── config.rs    # Config file structures (Settings, InstalledPlugins, ConfigPaths)
│   ├── discovery.rs # PluginDiscovery: scans installed plugins from Claude config
│   └── operations.rs # PluginService: enable/disable with file locking + atomic writes
└── ui/
    ├── mod.rs       # Main render function, header/footer layout
    ├── plugin_list.rs
    ├── details.rs
    ├── detail_modal.rs
    ├── dialogs.rs
    └── help.rs
```

### Key Concepts

**Three-Scope System**: Plugins can be installed and enabled at three scopes:

| Scope   | Settings File                   | installed_plugins.json fields        | Purpose                    |
|---------|---------------------------------|--------------------------------------|----------------------------|
| User    | `~/.claude/settings.json`       | `scope: "user"`                      | Global, all projects       |
| Project | `./.claude/settings.json`       | `scope: "project"`, `projectPath`    | Team-shared, in git        |
| Local   | `./.claude/settings.local.json` | `scope: "local"`, `projectPath`      | Personal, gitignored       |

Settings precedence: Local > Project > User

**Plugin Discovery** (`plugin/discovery.rs`):
- Reads `~/.claude/plugins/installed_plugins.json` for installation data
- Reads settings from all THREE scopes to determine enabled status
- Uses `projectPath` field to determine `is_current_project` for project/local scopes
- Merges into `Plugin` structs with `enabled_user`, `enabled_project`, and `enabled_local` fields

**Atomic Operations** (`plugin/operations.rs`):
- Uses `fs2` file locking for concurrent safety
- Writes via temp file + rename for atomicity

**App Modes** (`app.rs`):
- `Normal`: navigation and plugin actions
- `Search`: incremental filtering
- `Help`, `Confirm`, `DetailModal`: overlay states

**UI Features**:
- CWD displayed in header (format: `~/relative/path`)
- Scope indicators: `[U]`, `[P]`, `[P*]`, `[L]`, `[L*]`
- Project path shown for all project/local scope plugins

### Config Files Read

| File | Purpose |
|------|---------|
| `~/.claude/settings.json` | User-scope enabled plugins |
| `./.claude/settings.json` | Project-scope enabled plugins (team-shared) |
| `./.claude/settings.local.json` | Local-scope enabled plugins (personal) |
| `~/.claude/plugins/installed_plugins.json` | Installation metadata (includes `projectPath`) |
| `~/.claude/plugins/known_marketplaces.json` | Marketplace sources |
| `<install_path>/.claude-plugin/plugin.json` | Plugin manifest |

### Testing

Unit tests are co-located in each module. Integration tests in `tests/integration.rs` exercise CLI commands via `assert_cmd`.

MSRV is Rust 1.70.

## In-Progress Features

See `FEATURE_PLAN.md` for full details. Context files in `.claude/` directory.

### Scope Selection (Feature B)

**Problem**: Toggle uses `install_scope`, may create unwanted `settings.json`.

**Solution**:
- `ScopeSelectionMode` enum: Modal, Inline, Keybinding (compile-time const)
- Keybindings: `u`/`p`/`l` for direct scope enable
- `AppMode::ScopeSelect` for dialog state
- Enter triggers scope selection instead of direct toggle

**Key files**: `src/app.rs`, `src/ui/dialogs.rs`, `src/main.rs`

### Important Design Decision

**Installation vs Enabling are separate concerns:**
- `installed_plugins.json` tracks WHERE plugin files live (never modified on enable/disable)
- `settings.json` files track WHERE plugins are enabled (modified on enable/disable)

A plugin installed at any scope can be enabled at any scope independently.
