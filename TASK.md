# TASK: Implement Three-Scope Plugin Management for CCPM

## Objective

Extend CCPM to properly handle Claude Code's three configuration scopes (User, Project, Local) with accurate project-path awareness.

## Background

Claude Code uses three distinct configuration scopes:

| Scope   | Settings File                   | installed_plugins.json fields        | Purpose                    |
|---------|---------------------------------|--------------------------------------|----------------------------|
| User    | `~/.claude/settings.json`       | `scope: "user"`                      | Global, all projects       |
| Project | `./.claude/settings.json`       | `scope: "project"`, `projectPath`    | Team-shared, in git        |
| Local   | `./.claude/settings.local.json` | `scope: "local"`, `projectPath`      | Personal, gitignored       |

### Key Insight: projectPath Field

For project and local scopes, `installed_plugins.json` stores a `projectPath` field indicating which directory the plugin was installed in. This is critical for proper filtering.

### Known Bug We're Fixing

Claude Code Issue #14202: Project-scoped plugins incorrectly show as installed globally because the UI doesn't check `projectPath`. CCPM will do this correctly.

## Requirements

### Phase 1: Core Scope Infrastructure

#### 1.1 Update Scope Enum (`src/plugin/mod.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    #[default]
    User,
    Project,
    Local,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::User => write!(f, "user"),
            Scope::Project => write!(f, "project"),
            Scope::Local => write!(f, "local"),
        }
    }
}
```

Update `scope_indicator()`:
```rust
pub fn scope_indicator(&self) -> &'static str {
    match (self.install_scope, self.is_current_project) {
        (Scope::User, _) => "[U]",
        (Scope::Project, true) => "[P]",
        (Scope::Project, false) => "[P*]",
        (Scope::Local, true) => "[L]",
        (Scope::Local, false) => "[L*]",
    }
}
```

#### 1.2 Update ScopeFilter (`src/plugin/mod.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScopeFilter {
    #[default]
    All,
    User,
    Project,
    Local,
}

impl ScopeFilter {
    pub fn next(&self) -> Self {
        match self {
            ScopeFilter::All => ScopeFilter::User,
            ScopeFilter::User => ScopeFilter::Project,
            ScopeFilter::Project => ScopeFilter::Local,
            ScopeFilter::Local => ScopeFilter::All,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ScopeFilter::All => "All",
            ScopeFilter::User => "User",
            ScopeFilter::Project => "Project",
            ScopeFilter::Local => "Local",
        }
    }
}
```

#### 1.3 Update Plugin Struct (`src/plugin/mod.rs`)

```rust
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub marketplace: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<Author>,

    // Installation information
    pub install_scope: Scope,
    pub install_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,  // NEW: The project this was installed in
    pub is_current_project: bool,

    // Enabled status per scope
    pub enabled_user: bool,
    pub enabled_project: bool,  // NEW
    pub enabled_local: bool,

    pub installed_at: Option<String>,
    pub last_updated: Option<String>,
}
```

Update `is_enabled()` with precedence (Local > Project > User):
```rust
pub fn is_enabled(&self) -> bool {
    // Local overrides Project overrides User
    if self.enabled_local {
        return true;
    }
    if self.enabled_project {
        return true;
    }
    self.enabled_user
}
```

Update `enabled_context()`:
```rust
pub fn enabled_context(&self) -> String {
    let mut contexts = Vec::new();
    if self.enabled_user { contexts.push("User"); }
    if self.enabled_project { contexts.push("Project"); }
    if self.enabled_local { contexts.push("Local"); }

    if contexts.is_empty() {
        "Disabled".to_string()
    } else {
        contexts.join(" + ")
    }
}
```

Add helper for displaying project path relative to home:
```rust
pub fn project_path_display(&self) -> Option<String> {
    self.project_path.as_ref().map(|p| {
        if let Some(home) = dirs::home_dir() {
            if let Ok(relative) = p.strip_prefix(&home) {
                return format!("~/{}", relative.display());
            }
        }
        p.display().to_string()
    })
}
```

### Phase 2: Config Updates

#### 2.1 Update ConfigPaths (`src/plugin/config.rs`)

```rust
impl ConfigPaths {
    // ... existing methods ...

    pub fn project_settings(&self) -> PathBuf {
        self.local_dir.join("settings.json")
    }

    pub fn local_settings(&self) -> PathBuf {
        self.local_dir.join("settings.local.json")
    }
}
```

#### 2.2 Update InstalledPluginEntry (`src/plugin/config.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPluginEntry {
    pub scope: String,
    pub install_path: PathBuf,
    #[serde(default)]
    pub project_path: Option<PathBuf>,  // NEW: For project/local scopes
    pub version: String,
    pub installed_at: String,
    pub last_updated: String,
    #[serde(default)]
    pub git_commit_sha: Option<String>,
    #[serde(default)]
    pub is_local: bool,
}
```

### Phase 3: Discovery Updates

#### 3.1 Update Plugin Discovery (`src/plugin/discovery.rs`)

Read from THREE settings files:
```rust
pub fn discover_all(&self) -> Result<Vec<Plugin>> {
    // Load all three settings files
    let user_settings = self.load_settings(&self.paths.user_settings());
    let project_settings = self.load_settings(&self.paths.project_settings());
    let local_settings = self.load_settings(&self.paths.local_settings());

    // Track enabled status from ALL scopes
    let mut user_enabled: HashMap<String, bool> = HashMap::new();
    let mut project_enabled: HashMap<String, bool> = HashMap::new();
    let mut local_enabled: HashMap<String, bool> = HashMap::new();

    for (id, enabled) in &user_settings.enabled_plugins {
        user_enabled.insert(id.clone(), *enabled);
    }
    for (id, enabled) in &project_settings.enabled_plugins {
        project_enabled.insert(id.clone(), *enabled);
    }
    for (id, enabled) in &local_settings.enabled_plugins {
        local_enabled.insert(id.clone(), *enabled);
    }

    // ... rest of discovery logic
}
```

Parse scope with three options:
```rust
let install_scope = match entry.scope.as_str() {
    "project" => Scope::Project,
    "local" => Scope::Local,
    _ => Scope::User,
};
```

Check `projectPath` for project/local scopes:
```rust
let is_current_project = match install_scope {
    Scope::User => true,  // User scope is always "current"
    Scope::Project | Scope::Local => {
        // Check if projectPath matches current working directory
        if let Some(ref project_path) = entry.project_path {
            if let Ok(cwd) = env::current_dir() {
                project_path == &cwd
            } else {
                false
            }
        } else {
            // Fallback to old behavior if no projectPath
            self.is_local_install_current_project(&entry.install_path)
        }
    }
};
```

### Phase 4: UI Updates

#### 4.1 Add CWD Display

In header or footer (`src/ui/mod.rs`), show current working directory:
```rust
// In footer or header
let cwd = std::env::current_dir()
    .map(|p| {
        if let Some(home) = dirs::home_dir() {
            if let Ok(rel) = p.strip_prefix(&home) {
                return format!("~/{}", rel.display());
            }
        }
        p.display().to_string()
    })
    .unwrap_or_else(|_| "unknown".to_string());

// Display: "CWD: ~/projects/myapp"
```

#### 4.2 Update Details Panel (`src/ui/details.rs`)

Always show project path for Project and Local scopes (using relative-to-home format):
```rust
// Always show project path for project/local scopes
if plugin.install_scope != Scope::User {
    if let Some(path_display) = plugin.project_path_display() {
        lines.push(Line::from(vec![
            Span::styled("Project: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                path_display,
                if plugin.is_current_project {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            ),
        ]));
    }
}
```

Update enabled context to show all three scopes.

#### 4.3 Update Detail Modal (`src/ui/detail_modal.rs`)

Same changes as details panel.

### Phase 5: Operations Updates

#### 5.1 Update Plugin Operations (`src/plugin/operations.rs`)

Add `project_settings_path()` method and support enabling/disabling at project scope.

Update all pattern matches to handle three scopes.

### Phase 6: Testing & Documentation

#### 6.1 Update All Tests

- `src/plugin/mod.rs` - Add tests for Project scope, enabled_context with 3 scopes
- `src/plugin/config.rs` - Test project_settings() and local_settings() paths
- `src/plugin/discovery.rs` - Test projectPath parsing and is_current_project logic
- `tests/integration.rs` - Update CLI tests

#### 6.2 Update Documentation

After implementation:
- **CLAUDE.md**: Update architecture section with three-scope system
- **README.md**: Update user-facing docs with scope indicators and features

## Verification Commands

Run after each phase:
```bash
cargo check              # Type check
cargo clippy -- -D warnings  # Lint
cargo fmt --check        # Format check
cargo test               # All tests
```

## Success Criteria

1. `Scope` enum has 3 variants: User, Project, Local
2. Settings read from all 3 files: `settings.json` (user), `settings.json` (project), `settings.local.json` (local)
3. UI shows scope indicators: [U], [P], [L], [P*], [L*]
4. Project path shown for all project/local plugins (format: ~/relative/path)
5. Current working directory visible in UI
6. `projectPath` field properly parsed and used for is_current_project check
7. Scope filter cycles through 4 options (All, User, Project, Local)
8. All tests pass
9. No clippy warnings
10. Documentation updated (CLAUDE.md, README.md)

## Completion Signal

When all phases complete and verification passes, output:

<promise>SCOPE_IMPLEMENTATION_COMPLETE</promise>

## Out of Scope

These are explicitly NOT part of this task (see FEATURE_PLAN.md):
- `--plugin-dir` development plugins
- `--add-dir` directory access (not plugin-related)
- `enabledPlugins` pending installation display
- Marketplace browsing

## References

- [Claude Code Plugins Docs](https://code.claude.com/docs/en/plugins)
- [Claude Code Plugins Reference](https://code.claude.com/docs/en/plugins-reference)
- [Project-scope bug: Issue #14202](https://github.com/anthropics/claude-code/issues/14202)
