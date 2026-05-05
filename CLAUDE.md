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

## Git Safety (binding for Claude and all subagents)

Destructive git operations have caused real incidents in this project. The rule:

**Never run `git stash`, `git checkout <ref> -- <file>`, `git checkout -- <file>`, `git restore`, `git reset`, `git clean`, `git rebase`, `git revert`, `git push --force`, `git branch -D`, or `git worktree remove` without first asking the user.**

If a task appears to require any of these, stop and ask — describe what command you'd run, why, and what files it would affect. Wait for explicit approval. This applies to every subagent dispatched for this project too; plans that delegate work MUST cite this rule in the dispatch prompt.

**Allowed without asking:**
- Read-only git: `git status`, `git log`, `git diff`, `git show`, `git rev-parse`, `git ls-files`, `git blame`, `git branch --show-current`.
- `git add <specific paths>` and `git commit` for paths the user (or an approved plan) explicitly named. Never `git add -A` or `git add .`.
- Creating branches (`git checkout -b <name>`) on a clean working tree.

**Never silence errors** with `2>/dev/null`, `|| true`, or similar — if a command might fail, the failure must be visible.

This rule exists because subagents have proposed sequences like `git stash; git checkout <old-sha> -- tests/integration.rs; git stash pop 2>/dev/null` while reviewing changes unrelated to that file. The user caught it; they will not always catch it.

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

## CLI Commands

```bash
ccpm                     # Launch TUI
ccpm list                # List all plugins
ccpm list --debug        # List with debug info (Option values, project_path)
ccpm list -s local       # Filter by scope
ccpm list -e             # Show only enabled
ccpm enable <id>         # Enable plugin
ccpm disable <id>        # Disable plugin
ccpm info <id>           # Show plugin details
```

The `--debug` flag is useful for diagnosing settings-related issues. It outputs to stderr:
```
DEBUG: plugin-id -> user=Some(true) project=None local=Some(false) -> is_enabled=false project_path=Some("/path/to/project")
```

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
- For **all** plugins (including user-scope), `enabled_project` and `enabled_local` are populated from the CWD's `.claude/settings.json` and `.claude/settings.local.json` — so a CWD override on a globally-installed plugin is always respected
- For project/local scope plugins that have a `projectPath`: reads settings from the PLUGIN's `projectPath` directory to preserve cross-project isolation (a plugin installed in Project A won't show Project B's settings)
- Uses `projectPath` field to determine `is_current_project` for project/local scopes
- Caches settings per-project to avoid redundant file reads
- Merges into `Plugin` structs with `enabled_user`, `enabled_project`, and `enabled_local` fields

**Atomic Operations** (`plugin/operations.rs`):
- Uses `fs2` file locking for concurrent safety
- Writes via temp file + rename for atomicity
- Lock files contain PID/timestamp; stale/corrupted locks are auto-cleaned

**App Modes** (`app.rs`):
- `Normal`: navigation and plugin actions
- `Search`: incremental filtering
- `Help`, `Confirm`, `DetailModal`: overlay states

**UI Features**:
- CWD displayed in header (format: `~/relative/path`)
- Scope indicators: `[U]`, `[P]`, `[P*]`, `[L]`, `[L*]`
- Override marker `↓` (yellow) next to scope indicator when a non-install-scope setting is active
- `[overrides: N]` count in header when overrides exist in the current view
- Project path shown for all project/local scope plugins
- Details pane shows per-scope Settings block (User/Project/Local breakdown + Effective line)

### Config Files Read

| File | Purpose |
|------|---------|
| `~/.claude/settings.json` | User-scope enabled plugins |
| `./.claude/settings.json` | Project-scope enabled plugins (team-shared) |
| `./.claude/settings.local.json` | Local-scope enabled plugins (personal) |
| `~/.claude/plugins/installed_plugins.json` | Installation metadata (includes `projectPath`) |
| `~/.claude/plugins/known_marketplaces.json` | Marketplace sources |
| `<install_path>/.claude-plugin/plugin.json` | Plugin manifest |

### On-Disk JSON Format

CCPM-authored settings files use a deterministic alphabetical key order so a plugin toggle produces a minimal diff and files look identical across projects:
- `enabledPlugins` is the first top-level key (named struct field).
- All other top-level keys (flattened from `Settings.other`) follow in alphabetical order.
- Nested object keys are alphabetical at every level.
- Arrays (e.g. `permissions.allow`, `permissions.deny`, `permissions.ask`, hook arrays) preserve their existing order.
- Files end with a trailing newline.

Enforced at the type level via `BTreeMap` in `src/plugin/config.rs` (and `serde_json::Value::Object`'s default `BTreeMap` backing handles nested objects). CCPM does not modify `installed_plugins.json`, so its layout is untouched.

### Testing

Unit tests are co-located in each module. Integration tests in `tests/integration.rs` exercise CLI commands via `assert_cmd`.

MSRV is Rust 1.70.

## Important Design Decision

**Installation vs Enabling are separate concerns:**
- `installed_plugins.json` tracks WHERE plugin files live (never modified on enable/disable)
- `settings.json` files track WHERE plugins are enabled (modified on enable/disable)

A plugin installed at any scope can be enabled at any scope independently.
