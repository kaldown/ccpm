mod config;
mod discovery;
mod operations;

pub use config::*;
pub use discovery::*;
pub use operations::*;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    pub enabled_user: Option<bool>,    // Setting in ~/.claude/settings.json
    pub enabled_project: Option<bool>, // Setting in ./.claude/settings.json (project scope)
    pub enabled_local: Option<bool>,   // Setting in ./.claude/settings.local.json

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
}
