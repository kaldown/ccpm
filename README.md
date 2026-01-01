# CCPM - Claude Code Plugin Manager

[![CI](https://github.com/ccpm/ccpm/actions/workflows/ci.yml/badge.svg)](https://github.com/ccpm/ccpm/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

A terminal user interface (TUI) application for managing Claude Code plugins. Built with Rust and Ratatui, CCPM provides a lazygit-style experience for enabling, disabling, and managing your Claude Code plugins.

## Features

- **Interactive TUI**: Navigate and manage plugins with vim-style keybindings
- **Three-Scope Support**: Manage plugins at user, project, and local scope
- **Plugin Discovery**: Automatically discovers installed plugins from Claude Code configuration
- **Project Awareness**: Shows which project plugins are installed in via `projectPath`
- **CWD Display**: Header shows current working directory for context
- **Search & Filter**: Quickly find plugins by name, marketplace, or description
- **CLI Mode**: Non-interactive commands for scripting and automation
- **Safe Operations**: Atomic file writes and file locking for concurrent safety

## Installation

### From Source (Recommended)

```bash
cargo install --path .
```

### From Cargo

```bash
cargo install ccpm
```

### From Homebrew (macOS)

```bash
brew tap ccpm/homebrew-ccpm
brew install ccpm
```

## Usage

### TUI Mode

Launch the interactive interface:

```bash
ccpm
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to first |
| `G` | Go to last |
| `Enter` | View plugin details (modal) |
| `Space` | Toggle enable/disable |
| `e` | Enable plugin |
| `d` | Disable plugin |
| `s` | Cycle scope filter (All/User/Project/Local) |
| `/` | Start search |
| `Esc` | Clear search / Exit mode |
| `?` | Toggle help |
| `r` | Reload plugins |
| `q` | Quit |

### Scope Indicators

In the plugin list, each plugin shows a scope indicator:

| Indicator | Meaning |
|-----------|---------|
| `[U]` (blue) | User scope - installed in `~/.claude` (global) |
| `[P]` (cyan) | Project scope - installed in current project (shared in git) |
| `[P*]` (yellow) | Project scope - installed in a different project |
| `[L]` (magenta) | Local scope - installed in current project (gitignored) |
| `[L*]` (yellow) | Local scope - installed in a different project |

The detail panel shows:
- **Installed**: Where the plugin files are physically located
- **Enabled in**: Which settings files have the plugin enabled (User, Project, Local, or combinations)
- **Project**: For project/local scopes, shows the project path (format: `~/relative/path`)

### CLI Mode

List all plugins:
```bash
ccpm list
ccpm list --scope user
ccpm list --enabled
```

Example output:
```
NAME                           MARKETPLACE               STATUS   INSTALLED  ENABLED IN
------------------------------------------------------------------------------------------
context7                       claude-plugins-official   enabled  user       User only
agent-orchestration            claude-code-workflows     enabled  local      Local only
my-custom-plugin               local-dev                 disabled local*     Disabled
```

Enable/disable plugins:
```bash
ccpm enable plugin-name@marketplace
ccpm disable plugin-name@marketplace --scope local
```

Show plugin details:
```bash
ccpm info plugin-name@marketplace
```

Example output:
```
Name:        context7
Marketplace: claude-plugins-official
ID:          context7@claude-plugins-official
Status:      enabled
Installed:   User (~/.claude)
Enabled in:  User only
Version:     1.0.0
Path:        /Users/you/.claude/plugins/marketplaces/claude-plugins-official/context7
```

## Configuration

CCPM reads Claude Code configuration from three scopes:

| Scope | Settings File | Purpose |
|-------|--------------|---------|
| User | `~/.claude/settings.json` | Global settings, applies to all projects |
| Project | `./.claude/settings.json` | Team-shared settings, committed to git |
| Local | `./.claude/settings.local.json` | Personal settings, gitignored |

Plugin installation data is read from:
- `~/.claude/plugins/installed_plugins.json` (includes `projectPath` for project/local scopes)
- `~/.claude/plugins/known_marketplaces.json`

Settings precedence: Local > Project > User

## Building from Source

Requirements:
- Rust 1.70 or newer

```bash
git clone https://github.com/ccpm/ccpm
cd ccpm
cargo build --release
```

The binary will be at `target/release/ccpm`.

## Cross-Platform Support

CCPM works on:
- macOS (x86_64 and arm64)
- Linux (x86_64 and arm64)
- Windows (x86_64)

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
