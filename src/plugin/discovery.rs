use super::{
    config::{ConfigPaths, InstalledPlugins, KnownMarketplaces, PluginManifest, Settings},
    Author, Plugin, Result, Scope,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

pub struct PluginDiscovery {
    paths: ConfigPaths,
}

impl PluginDiscovery {
    pub fn new() -> Result<Self> {
        Ok(Self {
            paths: ConfigPaths::new()?,
        })
    }

    pub fn with_paths(paths: ConfigPaths) -> Self {
        Self { paths }
    }

    /// Discover all plugins from both user and local scopes
    pub fn discover_all(&self) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        // Load configuration files
        let user_settings = self.load_settings(&self.paths.user_settings());
        let local_settings = self.load_settings(&self.paths.local_settings());
        let installed = self.load_installed_plugins();
        let _marketplaces = self.load_known_marketplaces();

        // Track enabled status from BOTH scopes separately
        let mut user_enabled: HashMap<String, bool> = HashMap::new();
        let mut local_enabled: HashMap<String, bool> = HashMap::new();

        for (id, enabled) in &user_settings.enabled_plugins {
            user_enabled.insert(id.clone(), *enabled);
        }
        for (id, enabled) in &local_settings.enabled_plugins {
            local_enabled.insert(id.clone(), *enabled);
        }

        // Build plugin list from installed plugins
        for (id, entries) in &installed.plugins {
            if let Some(entry) = entries.first() {
                let (name, marketplace) = parse_plugin_id(id);
                let manifest = self.load_plugin_manifest(&entry.install_path);

                // Determine installation scope from entry.scope (source of truth)
                let install_scope = match entry.scope.as_str() {
                    "local" => Scope::Local,
                    _ => Scope::User,
                };

                // For local installs, check if it's the current project
                let is_current_project = if install_scope == Scope::Local {
                    self.is_local_install_current_project(&entry.install_path)
                } else {
                    true // User scope is always "current"
                };

                plugins.push(Plugin {
                    id: id.clone(),
                    name: manifest.as_ref().map(|m| m.name.clone()).unwrap_or(name),
                    marketplace,
                    description: manifest.as_ref().and_then(|m| m.description.clone()),
                    version: manifest
                        .as_ref()
                        .and_then(|m| m.version.clone())
                        .or_else(|| Some(entry.version.clone())),
                    author: manifest.as_ref().and_then(|m| {
                        m.author.as_ref().map(|a| Author {
                            name: a.name.clone(),
                            email: a.email.clone(),
                        })
                    }),
                    install_scope,
                    install_path: Some(entry.install_path.clone()),
                    is_current_project,
                    enabled_user: user_enabled.get(id).copied().unwrap_or(false),
                    enabled_local: local_enabled.get(id).copied().unwrap_or(false),
                    installed_at: Some(entry.installed_at.clone()),
                    last_updated: Some(entry.last_updated.clone()),
                });
            }
        }

        // Also include plugins that are in settings but not installed
        let all_ids: std::collections::HashSet<_> =
            user_enabled.keys().chain(local_enabled.keys()).collect();

        for id in all_ids {
            if !installed.plugins.contains_key(id) {
                let (name, marketplace) = parse_plugin_id(id);
                plugins.push(Plugin {
                    id: id.clone(),
                    name,
                    marketplace,
                    description: None,
                    version: None,
                    author: None,
                    install_scope: Scope::User, // Not installed, default to user
                    install_path: None,
                    is_current_project: true,
                    enabled_user: user_enabled.get(id).copied().unwrap_or(false),
                    enabled_local: local_enabled.get(id).copied().unwrap_or(false),
                    installed_at: None,
                    last_updated: None,
                });
            }
        }

        // Sort by name
        plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(plugins)
    }

    /// Check if a local install path is within the current working directory
    fn is_local_install_current_project(&self, install_path: &Path) -> bool {
        if let Ok(cwd) = env::current_dir() {
            // Check if install_path is under the current directory's .claude folder
            let current_claude_dir = cwd.join(".claude");
            install_path.starts_with(&current_claude_dir)
        } else {
            false
        }
    }

    /// Get marketplace info
    pub fn get_marketplaces(&self) -> HashMap<String, bool> {
        let marketplaces = self.load_known_marketplaces();
        marketplaces
            .marketplaces
            .iter()
            .map(|(name, entry)| (name.clone(), entry.auto_update))
            .collect()
    }

    fn load_settings(&self, path: &Path) -> Settings {
        if !path.exists() {
            return Settings::default();
        }

        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    fn load_installed_plugins(&self) -> InstalledPlugins {
        let path = self.paths.installed_plugins();
        if !path.exists() {
            return InstalledPlugins::default();
        }

        fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    fn load_known_marketplaces(&self) -> KnownMarketplaces {
        let path = self.paths.known_marketplaces();
        if !path.exists() {
            return KnownMarketplaces::default();
        }

        fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    fn load_plugin_manifest(&self, install_path: &Path) -> Option<PluginManifest> {
        let manifest_path = install_path.join(".claude-plugin").join("plugin.json");
        if !manifest_path.exists() {
            return None;
        }

        fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
    }
}

/// Parse plugin ID into (name, marketplace)
fn parse_plugin_id(id: &str) -> (String, String) {
    if let Some(pos) = id.rfind('@') {
        (id[..pos].to_string(), id[pos + 1..].to_string())
    } else {
        (id.to_string(), "unknown".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plugin_id() {
        let (name, marketplace) = parse_plugin_id("context7@claude-plugins-official");
        assert_eq!(name, "context7");
        assert_eq!(marketplace, "claude-plugins-official");
    }

    #[test]
    fn test_parse_plugin_id_no_at() {
        let (name, marketplace) = parse_plugin_id("some-plugin");
        assert_eq!(name, "some-plugin");
        assert_eq!(marketplace, "unknown");
    }
}
