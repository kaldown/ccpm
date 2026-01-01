# CCPM Feature Plan

This document tracks planned features and ideas for future development.

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

## Completed Features

- [x] Basic TUI plugin list
- [x] User/Local scope display (partial - needs Project scope)
- [x] Enable/disable plugins
- [x] Search/filter plugins
- [x] Detail modal

---

## References

- [Claude Code Plugins Docs](https://code.claude.com/docs/en/plugins)
- [Claude Code CLI Reference](https://code.claude.com/docs/en/cli-reference)
- [Project-scope bug: Issue #14202](https://github.com/anthropics/claude-code/issues/14202)
