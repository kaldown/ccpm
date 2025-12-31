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
    pub id: String,           // "name@marketplace"
    pub name: String,
    pub marketplace: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<Author>,
    pub enabled: bool,
    pub scope: Scope,
    pub install_path: Option<PathBuf>,
    pub installed_at: Option<DateTime>,
    pub last_updated: Option<DateTime>,
    pub auto_update: bool,
}

pub enum Scope {
    User,
    Local,
}

pub struct Author {
    pub name: String,
    pub email: Option<String>,
}
```

### State Management (Elm-like)

```rust
pub struct App {
    pub plugins: Vec<Plugin>,
    pub selected_index: usize,
    pub scope_filter: ScopeFilter,
    pub search_query: String,
    pub mode: AppMode,
    pub message: Option<StatusMessage>,
}

pub enum AppMode {
    Normal,
    Search,
    Help,
    Dialog(DialogKind),
}

pub enum Message {
    Navigate(Direction),
    ToggleEnable,
    ToggleAutoUpdate,
    SetScopeFilter(ScopeFilter),
    Search(String),
    ShowHelp,
    HideHelp,
    OpenDialog(DialogKind),
    CloseDialog,
    Confirm,
    Cancel,
    Quit,
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
├── MainLayout (horizontal split)
│   ├── PluginList (left panel)
│   │   ├── Filter bar
│   │   └── List items with indicators
│   └── DetailsPanel (right panel)
│       ├── Plugin info
│       └── Description
├── CommandBar (bottom)
└── Overlays
    ├── HelpOverlay
    └── DialogOverlay
```

### File Operations

- All writes use atomic operations (write to temp, rename)
- File locking with fs2 for concurrent access safety
- Graceful handling of missing/malformed files
