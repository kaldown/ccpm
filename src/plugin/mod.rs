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

    #[error("Home directory not found")]
    HomeDirNotFound,
}

pub type Result<T> = std::result::Result<T, PluginError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    #[default]
    User,
    Local,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::User => write!(f, "user"),
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
    pub is_current_project: bool, // For local: is it THIS project?

    // Enabled status (tracked separately for each scope)
    pub enabled_user: bool,  // Enabled in ~/.claude/settings.json
    pub enabled_local: bool, // Enabled in ./.claude/settings.json

    pub installed_at: Option<String>,
    pub last_updated: Option<String>,
}

impl Plugin {
    pub fn display_name(&self) -> String {
        format!("{}@{}", self.name, self.marketplace)
    }

    /// Returns true if the plugin is effectively enabled in the current context
    /// Local settings override user settings when present
    pub fn is_enabled(&self) -> bool {
        // If enabled in local scope, it's enabled
        if self.enabled_local {
            return true;
        }
        // If it's a local-only install and not enabled locally, check user
        self.enabled_user
    }

    /// Human-readable enabled context description
    pub fn enabled_context(&self) -> &'static str {
        match (self.enabled_user, self.enabled_local) {
            (true, true) => "User + Local",
            (true, false) => "User only",
            (false, true) => "Local only",
            (false, false) => "Disabled",
        }
    }

    /// Scope indicator for the list view: [U], [L], or [L*]
    pub fn scope_indicator(&self) -> &'static str {
        match (self.install_scope, self.is_current_project) {
            (Scope::User, _) => "[U]",
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScopeFilter {
    #[default]
    All,
    User,
    Local,
}

impl ScopeFilter {
    pub fn next(&self) -> Self {
        match self {
            ScopeFilter::All => ScopeFilter::User,
            ScopeFilter::User => ScopeFilter::Local,
            ScopeFilter::Local => ScopeFilter::All,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ScopeFilter::All => "All",
            ScopeFilter::User => "User",
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
        assert_eq!(Scope::Local.to_string(), "local");
    }

    #[test]
    fn test_scope_default() {
        assert_eq!(Scope::default(), Scope::User);
    }

    #[test]
    fn test_plugin_display_name() {
        let plugin = Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::User,
            install_path: None,
            is_current_project: true,
            enabled_user: true,
            enabled_local: false,
            installed_at: None,
            last_updated: None,
        };
        assert_eq!(plugin.display_name(), "test@marketplace");
    }

    #[test]
    fn test_plugin_status_indicator() {
        let mut plugin = Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::User,
            install_path: None,
            is_current_project: true,
            enabled_user: true,
            enabled_local: false,
            installed_at: None,
            last_updated: None,
        };

        assert_eq!(plugin.status_indicator(), "[+]");
        plugin.enabled_user = false;
        assert_eq!(plugin.status_indicator(), "[-]");
    }

    #[test]
    fn test_plugin_is_enabled() {
        let mut plugin = Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::User,
            install_path: None,
            is_current_project: true,
            enabled_user: false,
            enabled_local: false,
            installed_at: None,
            last_updated: None,
        };

        // Both disabled
        assert!(!plugin.is_enabled());

        // User enabled only
        plugin.enabled_user = true;
        assert!(plugin.is_enabled());

        // Local enabled overrides
        plugin.enabled_user = false;
        plugin.enabled_local = true;
        assert!(plugin.is_enabled());

        // Both enabled
        plugin.enabled_user = true;
        assert!(plugin.is_enabled());
    }

    #[test]
    fn test_plugin_enabled_context() {
        let mut plugin = Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::User,
            install_path: None,
            is_current_project: true,
            enabled_user: false,
            enabled_local: false,
            installed_at: None,
            last_updated: None,
        };

        assert_eq!(plugin.enabled_context(), "Disabled");

        plugin.enabled_user = true;
        assert_eq!(plugin.enabled_context(), "User only");

        plugin.enabled_local = true;
        assert_eq!(plugin.enabled_context(), "User + Local");

        plugin.enabled_user = false;
        assert_eq!(plugin.enabled_context(), "Local only");
    }

    #[test]
    fn test_plugin_scope_indicator() {
        let mut plugin = Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::User,
            install_path: None,
            is_current_project: true,
            enabled_user: false,
            enabled_local: false,
            installed_at: None,
            last_updated: None,
        };

        assert_eq!(plugin.scope_indicator(), "[U]");

        plugin.install_scope = Scope::Local;
        assert_eq!(plugin.scope_indicator(), "[L]");

        plugin.is_current_project = false;
        assert_eq!(plugin.scope_indicator(), "[L*]");
    }

    #[test]
    fn test_scope_filter_next() {
        assert_eq!(ScopeFilter::All.next(), ScopeFilter::User);
        assert_eq!(ScopeFilter::User.next(), ScopeFilter::Local);
        assert_eq!(ScopeFilter::Local.next(), ScopeFilter::All);
    }

    #[test]
    fn test_scope_filter_label() {
        assert_eq!(ScopeFilter::All.label(), "All");
        assert_eq!(ScopeFilter::User.label(), "User");
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
}
