mod config;
mod discovery;
mod operations;

pub use config::*;
pub use discovery::*;
pub use operations::*;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Failed to read config file: {path}")]
    ConfigReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse config file: {path}")]
    ConfigParseError {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("Failed to write config file: {path}")]
    ConfigWriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Marketplace not found: {0}")]
    MarketplaceNotFound(String),

    #[error("Failed to acquire file lock: {path}")]
    LockError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Lock file conflict: {path} (held by PID {pid})")]
    LockConflict { path: PathBuf, pid: u32 },

    #[error("Home directory not found")]
    HomeDirNotFound,
}

pub type Result<T> = std::result::Result<T, PluginError>;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub marketplace: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<Author>,

    // Installation information
    pub install_scope: Scope, // Where installed (from installed_plugins.json entry.scope)
    pub install_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>, // The project this was installed in (for project/local scopes)
    pub is_current_project: bool,      // For project/local: is it THIS project?

    // Enabled status (tracked separately for each scope)
    // None = no setting in that scope, Some(true) = enabled, Some(false) = disabled
    // Precedence: Local > Project > User (per Claude Code docs)
    pub enabled_user: Option<bool>, // Setting in ~/.claude/settings.json
    pub enabled_project: Option<bool>, // Setting in ./.claude/settings.json (project scope)
    pub enabled_local: Option<bool>, // Setting in ./.claude/settings.local.json

    pub installed_at: Option<String>,
    pub last_updated: Option<String>,
}

impl Plugin {
    pub fn display_name(&self) -> String {
        format!("{}@{}", self.name, self.marketplace)
    }

    /// Returns true if the plugin is effectively enabled in the current context
    /// Precedence: Local > Project > User (per Claude Code docs)
    /// If a scope has an explicit setting (Some), it wins over lower-priority scopes
    pub fn is_enabled(&self) -> bool {
        // Local setting wins if present (true OR false)
        if let Some(local) = self.enabled_local {
            return local;
        }
        // Project setting wins if present (true OR false)
        if let Some(project) = self.enabled_project {
            return project;
        }
        // Fall back to user setting, or false if no setting anywhere
        self.enabled_user.unwrap_or(false)
    }

    pub fn set_enabled(&mut self, scope: Scope, value: Option<bool>) {
        match scope {
            Scope::User => self.enabled_user = value,
            Scope::Project => self.enabled_project = value,
            Scope::Local => self.enabled_local = value,
        }
    }

    /// Human-readable enabled context description
    /// Shows which scopes have explicit settings and their values
    pub fn enabled_context(&self) -> String {
        let mut contexts = Vec::new();
        if let Some(true) = self.enabled_user {
            contexts.push("User");
        }
        if let Some(true) = self.enabled_project {
            contexts.push("Project");
        }
        if let Some(true) = self.enabled_local {
            contexts.push("Local");
        }

        if contexts.is_empty() {
            "Disabled".to_string()
        } else {
            contexts.join(" + ")
        }
    }

    /// Returns the scope that is determining the current enabled state
    /// Useful for showing which setting is "winning"
    pub fn effective_scope(&self) -> Option<&'static str> {
        if self.enabled_local.is_some() {
            Some("Local")
        } else if self.enabled_project.is_some() {
            Some("Project")
        } else if self.enabled_user.is_some() {
            Some("User")
        } else {
            None
        }
    }

    /// Scope indicator for the list view: [U], [P], [P*], [L], or [L*]
    pub fn scope_indicator(&self) -> &'static str {
        match (self.install_scope, self.is_current_project) {
            (Scope::User, _) => "[U]",
            (Scope::Project, true) => "[P]",
            (Scope::Project, false) => "[P*]", // Project but different directory
            (Scope::Local, true) => "[L]",
            (Scope::Local, false) => "[L*]", // Local but different project
        }
    }

    pub fn status_indicator(&self) -> &'static str {
        if self.is_enabled() {
            "[+]"
        } else {
            "[-]"
        }
    }

    /// Returns true when any non-install-scope `enabled_*` field is set.
    /// Used to render the "↓" override marker in the plugin list.
    pub fn has_override(&self) -> bool {
        match self.install_scope {
            Scope::User => self.enabled_project.is_some() || self.enabled_local.is_some(),
            Scope::Project => self.enabled_user.is_some() || self.enabled_local.is_some(),
            Scope::Local => self.enabled_user.is_some() || self.enabled_project.is_some(),
        }
    }

    /// Returns the project path formatted relative to home directory
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

    /// Path to the settings file that supplies this plugin's flag at the given scope.
    ///
    /// Returns `None` if no flag exists at the requested scope, or for `Scope::User`
    /// (the user-scope flag always lives in `~/.claude/settings.json` — not annotated
    /// by callers because that path carries no useful diagnostic information).
    ///
    /// For project/local-scope installs the project directory is `self.project_path`.
    /// For user-scope installs with a CWD override (project_path is None) the project
    /// directory falls back to `cwd`.
    pub fn project_settings_source(&self, scope: Scope, cwd: &Path) -> Option<PathBuf> {
        let has_value = match scope {
            Scope::User => return None,
            Scope::Project => self.enabled_project.is_some(),
            Scope::Local => self.enabled_local.is_some(),
        };
        if !has_value {
            return None;
        }

        let project_dir = self.project_path.as_deref().unwrap_or(cwd);
        let file_name = match scope {
            Scope::Project => "settings.json",
            Scope::Local => "settings.local.json",
            Scope::User => unreachable!(),
        };
        Some(project_dir.join(".claude").join(file_name))
    }

    /// Display-formatted source path with home-relative substitution (`~/...`).
    /// Returns `None` whenever `project_settings_source` would return `None`.
    pub fn project_settings_source_display(
        &self,
        scope: Scope,
        cwd: &Path,
    ) -> Option<String> {
        self.project_settings_source(scope, cwd).map(|p| {
            if let Some(home) = dirs::home_dir() {
                if let Ok(rel) = p.strip_prefix(&home) {
                    return format!("~/{}", rel.display());
                }
            }
            p.display().to_string()
        })
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_display() {
        assert_eq!(Scope::User.to_string(), "user");
        assert_eq!(Scope::Project.to_string(), "project");
        assert_eq!(Scope::Local.to_string(), "local");
    }

    #[test]
    fn test_scope_default() {
        assert_eq!(Scope::default(), Scope::User);
    }

    fn make_test_plugin() -> Plugin {
        Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::User,
            install_path: None,
            project_path: None,
            is_current_project: true,
            enabled_user: None,    // No setting
            enabled_project: None, // No setting
            enabled_local: None,   // No setting
            installed_at: None,
            last_updated: None,
        }
    }

    #[test]
    fn test_plugin_display_name() {
        let mut plugin = make_test_plugin();
        plugin.enabled_user = Some(true);
        assert_eq!(plugin.display_name(), "test@marketplace");
    }

    #[test]
    fn test_plugin_status_indicator() {
        let mut plugin = make_test_plugin();
        plugin.enabled_user = Some(true);

        assert_eq!(plugin.status_indicator(), "[+]");
        plugin.enabled_user = Some(false);
        assert_eq!(plugin.status_indicator(), "[-]");
        plugin.enabled_user = None;
        assert_eq!(plugin.status_indicator(), "[-]"); // No settings = disabled
    }

    #[test]
    fn test_plugin_is_enabled_basic() {
        let mut plugin = make_test_plugin();

        // No settings anywhere = disabled
        assert!(!plugin.is_enabled());

        // User enabled only
        plugin.enabled_user = Some(true);
        assert!(plugin.is_enabled());

        // User disabled explicitly
        plugin.enabled_user = Some(false);
        assert!(!plugin.is_enabled());
    }

    #[test]
    fn test_plugin_is_enabled_precedence() {
        let mut plugin = make_test_plugin();

        // Project enabled, no local = enabled
        plugin.enabled_project = Some(true);
        assert!(plugin.is_enabled());

        // Local enabled overrides project
        plugin.enabled_local = Some(true);
        assert!(plugin.is_enabled());

        // CRITICAL: Local DISABLED overrides Project ENABLED
        plugin.enabled_project = Some(true);
        plugin.enabled_local = Some(false);
        assert!(!plugin.is_enabled()); // Local wins!

        // Project disabled overrides User enabled
        plugin.enabled_user = Some(true);
        plugin.enabled_project = Some(false);
        plugin.enabled_local = None;
        assert!(!plugin.is_enabled()); // Project wins!
    }

    #[test]
    fn test_plugin_is_enabled_fallthrough() {
        let mut plugin = make_test_plugin();

        // No local, no project → user wins
        plugin.enabled_user = Some(true);
        assert!(plugin.is_enabled());

        // No local → project wins over user
        plugin.enabled_user = Some(true);
        plugin.enabled_project = Some(false);
        assert!(!plugin.is_enabled());

        // Local present → local wins
        plugin.enabled_user = Some(true);
        plugin.enabled_project = Some(true);
        plugin.enabled_local = Some(false);
        assert!(!plugin.is_enabled());
    }

    #[test]
    fn test_plugin_enabled_context() {
        let mut plugin = make_test_plugin();

        // No settings
        assert_eq!(plugin.enabled_context(), "Disabled");

        // User enabled
        plugin.enabled_user = Some(true);
        assert_eq!(plugin.enabled_context(), "User");

        // User + Project enabled
        plugin.enabled_project = Some(true);
        assert_eq!(plugin.enabled_context(), "User + Project");

        // All three enabled
        plugin.enabled_local = Some(true);
        assert_eq!(plugin.enabled_context(), "User + Project + Local");

        // User disabled, Project + Local enabled
        plugin.enabled_user = Some(false);
        assert_eq!(plugin.enabled_context(), "Project + Local");

        // Only Local enabled
        plugin.enabled_project = Some(false);
        assert_eq!(plugin.enabled_context(), "Local");

        // All explicitly disabled
        plugin.enabled_local = Some(false);
        assert_eq!(plugin.enabled_context(), "Disabled");
    }

    #[test]
    fn test_plugin_effective_scope() {
        let mut plugin = make_test_plugin();

        // No settings
        assert_eq!(plugin.effective_scope(), None);

        // Only user setting
        plugin.enabled_user = Some(true);
        assert_eq!(plugin.effective_scope(), Some("User"));

        // Project overrides user
        plugin.enabled_project = Some(false);
        assert_eq!(plugin.effective_scope(), Some("Project"));

        // Local overrides all
        plugin.enabled_local = Some(true);
        assert_eq!(plugin.effective_scope(), Some("Local"));
    }

    #[test]
    fn test_plugin_scope_indicator() {
        let mut plugin = make_test_plugin();

        // User scope
        assert_eq!(plugin.scope_indicator(), "[U]");

        // Project scope - current project
        plugin.install_scope = Scope::Project;
        assert_eq!(plugin.scope_indicator(), "[P]");

        // Project scope - different project
        plugin.is_current_project = false;
        assert_eq!(plugin.scope_indicator(), "[P*]");

        // Local scope - current project
        plugin.install_scope = Scope::Local;
        plugin.is_current_project = true;
        assert_eq!(plugin.scope_indicator(), "[L]");

        // Local scope - different project
        plugin.is_current_project = false;
        assert_eq!(plugin.scope_indicator(), "[L*]");
    }

    #[test]
    fn test_scope_filter_next() {
        assert_eq!(ScopeFilter::All.next(), ScopeFilter::User);
        assert_eq!(ScopeFilter::User.next(), ScopeFilter::Project);
        assert_eq!(ScopeFilter::Project.next(), ScopeFilter::Local);
        assert_eq!(ScopeFilter::Local.next(), ScopeFilter::All);
    }

    #[test]
    fn test_scope_filter_label() {
        assert_eq!(ScopeFilter::All.label(), "All");
        assert_eq!(ScopeFilter::User.label(), "User");
        assert_eq!(ScopeFilter::Project.label(), "Project");
        assert_eq!(ScopeFilter::Local.label(), "Local");
    }

    #[test]
    fn test_scope_filter_default() {
        assert_eq!(ScopeFilter::default(), ScopeFilter::All);
    }

    #[test]
    fn test_has_override_user_install() {
        let mut plugin = make_test_plugin();
        plugin.install_scope = Scope::User;

        // No settings anywhere → no override
        assert!(!plugin.has_override());

        // Only enabled_user (the install scope) → no override
        plugin.enabled_user = Some(true);
        assert!(!plugin.has_override());

        // Add Project setting → override
        plugin.enabled_project = Some(false);
        assert!(plugin.has_override());

        // Reset, add Local setting → override
        plugin.enabled_project = None;
        plugin.enabled_local = Some(true);
        assert!(plugin.has_override());
    }

    #[test]
    fn test_has_override_local_install() {
        let mut plugin = make_test_plugin();
        plugin.install_scope = Scope::Local;
        plugin.enabled_local = Some(true);

        // Only the install scope is set → no override
        assert!(!plugin.has_override());

        // User setting is an override for a Local-installed plugin
        plugin.enabled_user = Some(true);
        assert!(plugin.has_override());
    }

    #[test]
    fn test_plugin_error_display() {
        let err = PluginError::NotFound("test-plugin".to_string());
        assert_eq!(err.to_string(), "Plugin not found: test-plugin");

        let err = PluginError::MarketplaceNotFound("test-marketplace".to_string());
        assert_eq!(err.to_string(), "Marketplace not found: test-marketplace");

        let err = PluginError::HomeDirNotFound;
        assert_eq!(err.to_string(), "Home directory not found");
    }

    #[test]
    fn test_project_path_display() {
        let mut plugin = make_test_plugin();

        // No project path
        assert_eq!(plugin.project_path_display(), None);

        // With project path (non-home path)
        plugin.project_path = Some(PathBuf::from("/some/absolute/path"));
        assert_eq!(
            plugin.project_path_display(),
            Some("/some/absolute/path".to_string())
        );

        // With home-relative path (if we can get home dir)
        if let Some(home) = dirs::home_dir() {
            plugin.project_path = Some(home.join("projects/myapp"));
            assert_eq!(
                plugin.project_path_display(),
                Some("~/projects/myapp".to_string())
            );
        }
    }

    #[test]
    fn test_project_settings_source_local_install_with_project_path() {
        let mut plugin = make_test_plugin();
        plugin.install_scope = Scope::Local;
        plugin.project_path = Some(PathBuf::from("/proj"));
        plugin.enabled_local = Some(true);

        let cwd = PathBuf::from("/cwd");
        assert_eq!(
            plugin.project_settings_source(Scope::Local, &cwd),
            Some(PathBuf::from("/proj/.claude/settings.local.json"))
        );
    }

    #[test]
    fn test_project_settings_source_user_install_with_cwd_override() {
        let mut plugin = make_test_plugin();
        plugin.install_scope = Scope::User;
        plugin.project_path = None;
        plugin.enabled_local = Some(false);

        let cwd = PathBuf::from("/cwd");
        assert_eq!(
            plugin.project_settings_source(Scope::Local, &cwd),
            Some(PathBuf::from("/cwd/.claude/settings.local.json"))
        );
    }

    #[test]
    fn test_project_settings_source_user_scope_returns_none() {
        let mut plugin = make_test_plugin();
        plugin.enabled_user = Some(true);
        let cwd = PathBuf::from("/cwd");
        assert_eq!(plugin.project_settings_source(Scope::User, &cwd), None);
    }

    #[test]
    fn test_project_settings_source_no_value_returns_none() {
        let plugin = make_test_plugin(); // all enabled_* are None
        let cwd = PathBuf::from("/cwd");
        assert_eq!(plugin.project_settings_source(Scope::Project, &cwd), None);
        assert_eq!(plugin.project_settings_source(Scope::Local, &cwd), None);

        let mut user_plugin = make_test_plugin();
        user_plugin.enabled_user = Some(true);
        assert_eq!(
            user_plugin.project_settings_source(Scope::User, &cwd),
            None,
            "Scope::User must always return None even when enabled_user is set"
        );
    }

    #[test]
    fn test_project_settings_source_project_scope_uses_settings_json() {
        let mut plugin = make_test_plugin();
        plugin.install_scope = Scope::Project;
        plugin.project_path = Some(PathBuf::from("/proj"));
        plugin.enabled_project = Some(true);

        let cwd = PathBuf::from("/cwd");
        assert_eq!(
            plugin.project_settings_source(Scope::Project, &cwd),
            Some(PathBuf::from("/proj/.claude/settings.json"))
        );
    }

    #[test]
    fn test_project_settings_source_local_install_without_project_path_falls_back_to_cwd() {
        let mut plugin = make_test_plugin();
        plugin.install_scope = Scope::Local;
        plugin.project_path = None; // legacy data pre-projectPath fix
        plugin.enabled_local = Some(true);

        let cwd = PathBuf::from("/cwd");
        assert_eq!(
            plugin.project_settings_source(Scope::Local, &cwd),
            Some(PathBuf::from("/cwd/.claude/settings.local.json"))
        );
    }

    #[test]
    fn test_project_settings_source_display_returns_some_path_string() {
        let mut plugin = make_test_plugin();
        plugin.install_scope = Scope::Local;
        plugin.project_path = Some(PathBuf::from("/some/absolute/proj"));
        plugin.enabled_local = Some(true);

        let cwd = PathBuf::from("/cwd");
        assert_eq!(
            plugin.project_settings_source_display(Scope::Local, &cwd),
            Some("/some/absolute/proj/.claude/settings.local.json".to_string())
        );
    }

    #[test]
    fn test_project_settings_source_display_uses_home_relative_format() {
        if let Some(home) = dirs::home_dir() {
            let mut plugin = make_test_plugin();
            plugin.install_scope = Scope::Local;
            plugin.project_path = Some(home.join("Projects/myapp"));
            plugin.enabled_local = Some(true);

            let cwd = PathBuf::from("/cwd");
            assert_eq!(
                plugin.project_settings_source_display(Scope::Local, &cwd),
                Some("~/Projects/myapp/.claude/settings.local.json".to_string())
            );
        }
    }

    #[test]
    fn test_project_settings_source_display_returns_none_when_source_none() {
        let plugin = make_test_plugin(); // no values set
        let cwd = PathBuf::from("/cwd");
        assert_eq!(
            plugin.project_settings_source_display(Scope::Local, &cwd),
            None
        );
        assert_eq!(
            plugin.project_settings_source_display(Scope::User, &cwd),
            None
        );
    }
}
