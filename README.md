# CCPM - Claude Code Plugin Manager

[![CI](https://github.com/ccpm/ccpm/actions/workflows/ci.yml/badge.svg)](https://github.com/ccpm/ccpm/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

A terminal user interface (TUI) application for managing Claude Code plugins. Built with Rust and Ratatui, CCPM provides a lazygit-style experience for enabling, disabling, and managing your Claude Code plugins.

## Features

- **Interactive TUI**: Navigate and manage plugins with vim-style keybindings
- **Multi-scope Support**: Manage plugins at user and local (project) scope
- **Plugin Discovery**: Automatically discovers installed plugins from Claude Code configuration
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
| `e` | Enable plugin |
| `d` | Disable plugin |
| `Space` / `Enter` | Toggle enable/disable |
| `s` | Cycle scope filter (All/User/Local) |
| `/` | Start search |
| `Esc` | Clear search / Exit mode |
| `?` | Toggle help |
| `r` | Reload plugins |
| `q` | Quit |

### CLI Mode

List all plugins:
```bash
ccpm list
ccpm list --scope user
ccpm list --enabled
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

## Configuration

CCPM reads Claude Code configuration from:

- **User scope**: `~/.claude/settings.json`
- **Local scope**: `./.claude/settings.json`

Plugin installation data is read from:
- `~/.claude/plugins/installed_plugins.json`
- `~/.claude/plugins/known_marketplaces.json`

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
