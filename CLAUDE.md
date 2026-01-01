# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

**Dual Scope System**: Plugins can be installed and enabled at two scopes:
- **User scope**: `~/.claude/settings.json` - applies globally
- **Local scope**: `./.claude/settings.json` - applies to current project only

**Plugin Discovery** (`plugin/discovery.rs`):
- Reads `~/.claude/plugins/installed_plugins.json` for installation data
- Reads settings from both scopes to determine enabled status
- Merges into `Plugin` structs with `enabled_user` and `enabled_local` fields

**Atomic Operations** (`plugin/operations.rs`):
- Uses `fs2` file locking for concurrent safety
- Writes via temp file + rename for atomicity

**App Modes** (`app.rs`):
- `Normal`: navigation and plugin actions
- `Search`: incremental filtering
- `Help`, `Confirm`, `DetailModal`: overlay states

### Config Files Read

| File | Purpose |
|------|---------|
| `~/.claude/settings.json` | User-scope enabled plugins |
| `./.claude/settings.json` | Local-scope enabled plugins |
| `~/.claude/plugins/installed_plugins.json` | Installation metadata |
| `~/.claude/plugins/known_marketplaces.json` | Marketplace sources |
| `<install_path>/.claude-plugin/plugin.json` | Plugin manifest |

### Testing

Unit tests are co-located in each module. Integration tests in `tests/integration.rs` exercise CLI commands via `assert_cmd`.

MSRV is Rust 1.70.
