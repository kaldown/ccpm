# CCPM Architecture

## Claude Code Plugin System Overview

### File Locations

- **User Scope**: `~/.claude/`
  - `settings.json` - Contains `enabledPlugins` map
  - `plugins/installed_plugins.json` - Tracks installed plugins with metadata
  - `plugins/known_marketplaces.json` - Tracks marketplace sources
  - `plugins/cache/` - Cached plugin files
  - `plugins/marketplaces/` - Marketplace repositories

- **Local Scope**: `./.claude/`
  - `settings.json` - Project-specific plugin settings
  - `settings.local.json` - Local overrides (gitignored)

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
      "scope": "user",
      "installPath": "/path/to/plugin",
      "version": "1.0.0",
      "installedAt": "ISO8601",
      "lastUpdated": "ISO8601",
      "gitCommitSha": "sha",
      "isLocal": true
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
    pub is_current_project: bool, // For local: is it THIS project?

    // Enabled status (tracked separately for each scope)
    pub enabled_user: bool,      // Enabled in ~/.claude/settings.json
    pub enabled_local: bool,     // Enabled in ./.claude/settings.json

    pub installed_at: Option<String>,
    pub last_updated: Option<String>,
}

impl Plugin {
    /// Returns true if effectively enabled in current context
    pub fn is_enabled(&self) -> bool;

    /// "User only" | "Local only" | "User + Local" | "Disabled"
    pub fn enabled_context(&self) -> &'static str;

    /// "[U]" | "[L]" | "[L*]" (local in different project)
    pub fn scope_indicator(&self) -> &'static str;
}

pub enum Scope {
    User,   // Installed in ~/.claude
    Local,  // Installed in project's .claude
}

pub struct Author {
    pub name: String,
    pub email: Option<String>,
}
```

### Scope Detection Logic

The plugin scope is determined from `installed_plugins.json`, not from which `settings.json` has it enabled:

1. **Installation scope** (`install_scope`): Read from `entry.scope` in `installed_plugins.json`
2. **Current project detection** (`is_current_project`): For local installs, compare `install_path` with current working directory
3. **Enabled status**: Tracked separately for user (`~/.claude/settings.json`) and local (`./.claude/settings.json`)

This allows accurate display of:
- Where a plugin is physically installed
- Whether a local plugin belongs to the current project or another project
- Which settings files have the plugin enabled

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

pub enum AppMode {
    Normal,      // Default navigation mode
    Search,      // Search input active
    Help,        // Help overlay visible
    Confirm(ConfirmAction),  // Confirmation dialog
    DetailModal, // Full-screen plugin details
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
│   └── Scope filter, enabled count, search query
├── MainLayout (horizontal split 50/50)
│   ├── PluginList (left panel)
│   │   └── List items with [U]/[L]/[L*] scope + [+]/[-] status indicators
│   └── DetailsPanel (right panel)
│       ├── Plugin info (name, marketplace, status)
│       ├── Installed location & enabled context
│       ├── Version, author, path
│       └── Description
├── CommandBar (bottom)
│   └── Mode-specific keybinding hints + status messages
└── Overlays (modal dialogs)
    ├── HelpOverlay (? key)
    ├── ConfirmDialog (x key for remove)
    └── DetailModal (Enter key - expanded plugin info)
```

### File Operations

- All writes use atomic operations (write to temp, rename)
- File locking with fs2 for concurrent access safety
- Graceful handling of missing/malformed files

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
