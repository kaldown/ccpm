# CCPM Architecture

## Claude Code Plugin System Overview

### File Locations

- **User Scope**: `~/.claude/`
  - `settings.json` - Contains `enabledPlugins` map
  - `plugins/installed_plugins.json` - Tracks installed plugins with metadata
  - `plugins/known_marketplaces.json` - Tracks marketplace sources
  - `plugins/cache/` - Cached plugin files
  - `plugins/marketplaces/` - Marketplace repositories

- **Project Scope**: `./.claude/`
  - `settings.json` - Project-specific plugin settings (team-shared, committed to git)

- **Local Scope**: `./.claude/`
  - `settings.local.json` - Personal per-project overrides (gitignored)

### Data Structures

#### settings.json
```json
{
  "enabledPlugins": {
    "plugin-name@marketplace": true/false
  }
}
```

#### installed_plugins.json
```json
{
  "version": 2,
  "plugins": {
    "plugin-name@marketplace": [{
      "scope": "user",          // "user" | "project" | "local"
      "installPath": "/path/to/plugin",
      "version": "1.0.0",
      "installedAt": "ISO8601",
      "lastUpdated": "ISO8601",
      "gitCommitSha": "sha",
      "isLocal": true,
      "projectPath": "/path/to/project"  // present for project/local scope only
    }]
  }
}
```

#### known_marketplaces.json
```json
{
  "marketplace-name": {
    "source": {
      "source": "github",
      "repo": "owner/repo"
    },
    "installLocation": "/path",
    "lastUpdated": "ISO8601",
    "autoUpdate": true
  }
}
```

#### plugin.json (per plugin)
```json
{
  "name": "plugin-name",
  "description": "Plugin description",
  "version": "1.0.0",
  "author": {
    "name": "Author",
    "email": "email@example.com"
  },
  "mcpServers": {
    "server-name": {
      "command": "command",
      "args": ["args"]
    }
  }
}
```

## Application Architecture

### Core Data Models

```rust
pub struct Plugin {
    pub id: String,              // "name@marketplace"
    pub name: String,
    pub marketplace: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<Author>,

    // Installation information
    pub install_scope: Scope,    // Where installed (from installed_plugins.json)
    pub install_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,  // For project/local: which project owns this install
    pub is_current_project: bool, // For project/local: is this the CWD project?

    // Enabled status (tracked separately for each scope)
    // Option semantics: None = no setting, Some(true) = enabled, Some(false) = disabled
    // Precedence: Local > Project > User (per Claude Code docs)
    pub enabled_user: Option<bool>,     // Setting in ~/.claude/settings.json
    pub enabled_project: Option<bool>,  // Setting in ./.claude/settings.json (CWD)
    pub enabled_local: Option<bool>,    // Setting in ./.claude/settings.local.json (CWD)

    pub installed_at: Option<String>,
    pub last_updated: Option<String>,
}

impl Plugin {
    /// Returns true if effectively enabled in current context
    /// Precedence: Local > Project > User
    /// If a scope has Some(_), it wins over lower-priority scopes
    pub fn is_enabled(&self) -> bool;

    /// Human-readable context: "User + Project" | "Local" | "Disabled"
    pub fn enabled_context(&self) -> String;

    /// Which scope is determining the current state
    pub fn effective_scope(&self) -> Option<&'static str>;

    /// "[U]" | "[P]" | "[P*]" | "[L]" | "[L*]"
    pub fn scope_indicator(&self) -> &'static str;

    /// Sets the enabled_* field for a specific scope (None clears it)
    pub fn set_enabled(&mut self, scope: Scope, value: Option<bool>);

    /// True when any scope other than install_scope has a setting for this plugin
    pub fn has_override(&self) -> bool;
}

pub enum Scope {
    User,    // Installed in ~/.claude (global)
    Project, // Installed in project's .claude (team-shared, committed)
    Local,   // Installed in project's .claude (personal, gitignored)
}

pub struct Author {
    pub name: String,
    pub email: Option<String>,
}
```

### Scope Detection Logic

The plugin scope is determined from `installed_plugins.json`, not from which `settings.json` has it enabled:

1. **Installation scope** (`install_scope`): Read from `entry.scope` in `installed_plugins.json`
2. **Current project detection** (`is_current_project`): For project/local installs, compare `entry.project_path` with current working directory
3. **Enabled status** (`enabled_project`, `enabled_local`): **always populated from CWD** — so a CWD override always applies regardless of install scope

### Cross-Project Settings Isolation

Project/local-scope plugins installed in other projects read their enabled state from their own project's settings, not CWD:

```
Plugin: agent-orchestration@marketplace
Install scope: local
Project path: ~/Projects/Ternv3

When CCPM runs from ~/Projects/ccpm:
  - enabled_user:    from ~/.claude/settings.json       (always)
  - enabled_project: from ~/Projects/Ternv3/.claude/settings.json (plugin's own project)
  - enabled_local:   from ~/Projects/Ternv3/.claude/settings.local.json (plugin's own project)
```

This ensures that a plugin installed in Project A shows the correct on/off state when CCPM is opened from Project B.

User-scope plugins follow the CWD-only path (no `project_path` to look up):

```
Plugin: gitlab@claude-plugins-official
Install scope: user

When CCPM runs from ~/Projects/ccpm:
  - enabled_user:    from ~/.claude/settings.json (global)
  - enabled_project: from ~/Projects/ccpm/.claude/settings.json (CWD)
  - enabled_local:   from ~/Projects/ccpm/.claude/settings.local.json (CWD)
```

Writing `Local = false` for a user-scope plugin from the right project directory is sufficient to disable it in that project only.

### Settings Loading Strategy

```rust
// Always load CWD project/local settings (used for user-scope plugins and fallback)
let cwd_project_enabled = cwd_project_settings.map(|s| s.enabled_plugins).unwrap_or_default();
let cwd_local_enabled   = cwd_local_settings.map(|s| s.enabled_plugins).unwrap_or_default();

// Cache to avoid re-reading cross-project settings
let mut project_settings_cache: HashMap<PathBuf, (Option<Settings>, Option<Settings>)>;

// For each plugin:
let (plugin_enabled_project, plugin_enabled_local) = match install_scope {
    Scope::User => {
        // User-scope: CWD settings are the override source
        (
            cwd_project_enabled.get(id).copied(),
            cwd_local_enabled.get(id).copied(),
        )
    }
    Scope::Project | Scope::Local => {
        if let Some(ref proj_path) = entry.project_path {
            // Read from plugin's own project_path for cross-project isolation
            let (proj, local) = cache.entry(proj_path.clone())
                .or_insert_with(|| ConfigPaths::load_settings_from_project(proj_path));
            (
                proj.as_ref().and_then(|s| s.enabled_plugins.get(id).copied()),
                local.as_ref().and_then(|s| s.enabled_plugins.get(id).copied()),
            )
        } else {
            // No project_path: fall back to CWD
            (
                cwd_project_enabled.get(id).copied(),
                cwd_local_enabled.get(id).copied(),
            )
        }
    }
};
```

This allows accurate display of:
- Where a plugin is physically installed
- Whether a project/local plugin belongs to the current project or another project
- Which settings files have the plugin enabled, including CWD overrides on user-scope plugins

### State Management (Elm-like)

```rust
pub struct App {
    pub plugins: Vec<Plugin>,
    pub filtered_plugins: Vec<usize>,
    pub selected_index: usize,
    pub scope_filter: ScopeFilter,
    pub search_query: String,
    pub mode: AppMode,
    pub message: Option<StatusMessage>,
    pub service: PluginService,
}

impl App {
    /// Toggle plugin at an explicit scope (Local / Project / User).
    /// None branch: first press flips the effective state.
    pub fn toggle_selected_at_scope(&mut self, scope: Scope);

    /// Enable plugin at an explicit scope.
    pub fn enable_selected_at_scope(&mut self, scope: Scope);

    /// Disable plugin at an explicit scope.
    pub fn disable_selected_at_scope(&mut self, scope: Scope);

    /// Count of plugins in the current filtered view that have any override.
    pub fn override_count(&self) -> usize;
}

pub enum AppMode {
    Normal,      // Default navigation mode
    Search,      // Search input active
    Help,        // Help overlay visible
    Confirm(ConfirmAction),  // Confirmation dialog
    DetailModal, // Full-screen plugin details (i key)
}

pub enum ConfirmAction {
    Remove,
}
```

### Service Layer

```rust
pub trait PluginService {
    fn discover_plugins(&self) -> Result<Vec<Plugin>>;
    fn enable_plugin(&mut self, id: &str, scope: Scope) -> Result<()>;
    fn disable_plugin(&mut self, id: &str, scope: Scope) -> Result<()>;
    /// Toggle at an explicit scope; first press flips effective state when no setting exists.
    fn toggle_at_scope(&self, plugin: &Plugin, scope: Scope) -> Result<bool>;
    fn toggle_auto_update(&mut self, marketplace: &str) -> Result<()>;
    fn add_plugin(&mut self, source: &str, scope: Scope) -> Result<Plugin>;
    fn remove_plugin(&mut self, id: &str) -> Result<()>;
    fn update_plugin(&mut self, id: &str) -> Result<()>;
}
```

### TUI Component Hierarchy

```
App
├── Header (status bar)
│   └── CWD, scope filter, enabled count, [overrides: N], search query
├── MainLayout (horizontal split 50/50)
│   ├── PluginList (left panel)
│   │   └── List items with [U]/[P]/[P*]/[L]/[L*] scope + ↓ override marker + [+]/[-] status
│   └── DetailsPanel (right panel)
│       ├── Plugin info (name, marketplace, status)
│       ├── Installed location & enabled context
│       ├── Settings block: per-scope breakdown (User/Project/Local) + Effective line
│       ├── Version, author, path
│       └── Description
├── CommandBar (bottom)
│   └── Mode-specific keybinding hints (Enter/l/p/u/e/d/i/…) + status messages
└── Overlays (modal dialogs)
    ├── HelpOverlay (? key) — includes per-scope keybinding table
    ├── ConfirmDialog (x key for remove)
    └── DetailModal (i key - expanded plugin info)
```

### File Operations

- All writes use atomic operations (write to temp, rename)
- File locking with fs2 for concurrent access safety
- Graceful handling of missing/malformed files
- **Deterministic on-disk JSON layout:** settings files are written with alphabetical key ordering (via `BTreeMap` in `Settings.enabled_plugins`, `Settings.other`, and `KnownMarketplaces.marketplaces`) and a trailing newline. This guarantees minimal diffs across plugin toggles and identical layout across projects. Nested object keys are also alphabetized via `serde_json::Value::Object`'s default `BTreeMap` backing; arrays preserve their meaningful order. The trailing newline is written before `sync_all` so it is fsynced as part of the atomic write.

#### Lock File Handling

Lock files are managed with vim-style cleanup behavior:

```rust
/// Metadata stored in lock files for debugging and stale detection
struct LockMetadata {
    pid: u32,
    timestamp: DateTime<Utc>,
}

/// Guard that auto-deletes lock file on Drop (normal completion or panic)
pub struct LockFileGuard {
    lock_path: PathBuf,
    _file: File, // Holds the exclusive lock
}
```

Lock file JSON format:
```json
{
  "pid": 12345,
  "timestamp": "2026-01-02T10:30:00Z"
}
```

Stale lock detection:
- On Unix: Uses `kill -0 $PID` to check if process is running
- On non-Unix: Conservatively assumes process is active
- If PID is dead, lock file is deleted and new lock is acquired
- If PID is active, returns `PluginError::LockConflict { path, pid }`
- Corrupted or empty lock files are treated as stale and auto-deleted
