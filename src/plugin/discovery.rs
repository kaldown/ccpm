use super::{
    config::{ConfigPaths, InstalledPlugins, KnownMarketplaces, PluginManifest, Settings},
    Author, Plugin, Result, Scope,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

    /// Discover all plugins from user, project, and local scopes
    pub fn discover_all(&self) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        // Load user settings (global, always from ~/.claude/settings.json)
        let user_settings = self.load_settings(&self.paths.user_settings());

        // Load CWD settings (for plugins NOT installed in other projects)
        let cwd_project_settings = self.load_settings(&self.paths.project_settings());
        let cwd_local_settings = self.load_settings(&self.paths.local_settings());

        let installed = self.load_installed_plugins();
        let _marketplaces = self.load_known_marketplaces();

        // User enabled status (global)
        let mut user_enabled: HashMap<String, bool> = HashMap::new();
        for (id, enabled) in &user_settings.enabled_plugins {
            user_enabled.insert(id.clone(), *enabled);
        }

        // CWD settings (for plugins without project_path or for the current project)
        let mut cwd_project_enabled: HashMap<String, bool> = HashMap::new();
        let mut cwd_local_enabled: HashMap<String, bool> = HashMap::new();
        for (id, enabled) in &cwd_project_settings.enabled_plugins {
            cwd_project_enabled.insert(id.clone(), *enabled);
        }
        for (id, enabled) in &cwd_local_settings.enabled_plugins {
            cwd_local_enabled.insert(id.clone(), *enabled);
        }

        // Cache for settings loaded from other project directories
        let mut project_settings_cache: HashMap<PathBuf, (Option<Settings>, Option<Settings>)> = HashMap::new();

        // Build plugin list from installed plugins
        for (id, entries) in &installed.plugins {
            if let Some(entry) = entries.first() {
                let (name, marketplace) = parse_plugin_id(id);
                let manifest = self.load_plugin_manifest(&entry.install_path);

                // Determine installation scope from entry.scope (source of truth)
                let install_scope = match entry.scope.as_str() {
                    "project" => Scope::Project,
                    "local" => Scope::Local,
                    _ => Scope::User,
                };

                // For project/local installs, check if it's the current project
                let is_current_project = match install_scope {
                    Scope::User => true, // User scope is always "current"
                    Scope::Project | Scope::Local => {
                        // Check projectPath field if available (preferred)
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

                // CRITICAL FIX: Load settings from the PLUGIN's project, not CWD
                let (plugin_enabled_project, plugin_enabled_local) = match install_scope {
                    Scope::User => {
                        // User scope: no project/local settings apply
                        (None, None)
                    }
                    Scope::Project | Scope::Local => {
                        // Project/Local scope: read from PLUGIN's project, not CWD
                        if let Some(ref proj_path) = entry.project_path {
                            // Get or load settings for this project
                            let (proj_settings, local_settings) = project_settings_cache
                                .entry(proj_path.clone())
                                .or_insert_with(|| ConfigPaths::load_settings_from_project(proj_path));

                            (
                                proj_settings.as_ref().and_then(|s| s.enabled_plugins.get(id).copied()),
                                local_settings.as_ref().and_then(|s| s.enabled_plugins.get(id).copied()),
                            )
                        } else {
                            // Fallback to CWD if no project_path (shouldn't happen for new installs)
                            (
                                cwd_project_enabled.get(id).copied(),
                                cwd_local_enabled.get(id).copied(),
                            )
                        }
                    }
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
                    project_path: entry.project_path.clone(),
                    is_current_project,
                    // Preserve Option semantics: None = no setting, Some = explicit setting
                    // This is critical for correct precedence (Local > Project > User)
                    enabled_user: user_enabled.get(id).copied(),
                    enabled_project: plugin_enabled_project,
                    enabled_local: plugin_enabled_local,
                    installed_at: Some(entry.installed_at.clone()),
                    last_updated: Some(entry.last_updated.clone()),
                });
            }
        }

        // Also include plugins that are in settings but not installed
        // These use CWD settings since they have no project_path
        let all_ids: std::collections::HashSet<_> = user_enabled
            .keys()
            .chain(cwd_project_enabled.keys())
            .chain(cwd_local_enabled.keys())
            .collect();

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
                    project_path: None,
                    is_current_project: true,
                    // Preserve Option semantics for correct precedence
                    enabled_user: user_enabled.get(id).copied(),
                    enabled_project: cwd_project_enabled.get(id).copied(),
                    enabled_local: cwd_local_enabled.get(id).copied(),
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
    use tempfile::TempDir;

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

    fn create_test_settings(dir: &Path, plugins: &[(&str, bool)]) {
        let claude_dir = dir.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();

        let mut enabled_plugins = serde_json::Map::new();
        for (id, enabled) in plugins {
            enabled_plugins.insert(id.to_string(), serde_json::Value::Bool(*enabled));
        }

        let settings = serde_json::json!({
            "enabledPlugins": enabled_plugins
        });

        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .unwrap();
    }

    fn create_test_local_settings(dir: &Path, plugins: &[(&str, bool)]) {
        let claude_dir = dir.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();

        let mut enabled_plugins = serde_json::Map::new();
        for (id, enabled) in plugins {
            enabled_plugins.insert(id.to_string(), serde_json::Value::Bool(*enabled));
        }

        let settings = serde_json::json!({
            "enabledPlugins": enabled_plugins
        });

        fs::write(
            claude_dir.join("settings.local.json"),
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_cross_project_settings_isolation() {
        // This test verifies that plugins installed in project_a
        // read their enabled state from project_a's settings,
        // NOT from the current working directory

        // Setup: Create two temp "projects"
        let project_a = TempDir::new().unwrap();
        let project_b = TempDir::new().unwrap();

        // Plugin installed in project_a with Local scope
        // project_a/.claude/settings.json has plugin: true
        // project_a/.claude/settings.local.json has plugin: false
        create_test_settings(project_a.path(), &[("test@marketplace", true)]);
        create_test_local_settings(project_a.path(), &[("test@marketplace", false)]);

        // project_b (simulating CWD) has DIFFERENT settings - plugin: true everywhere
        create_test_settings(project_b.path(), &[("test@marketplace", true)]);
        create_test_local_settings(project_b.path(), &[("test@marketplace", true)]);

        // Test loading settings from project_a
        let (proj_settings, local_settings) =
            ConfigPaths::load_settings_from_project(project_a.path());

        // Verify project_a's settings are read correctly
        let proj_enabled = proj_settings
            .as_ref()
            .and_then(|s| s.enabled_plugins.get("test@marketplace").copied());
        let local_enabled = local_settings
            .as_ref()
            .and_then(|s| s.enabled_plugins.get("test@marketplace").copied());

        assert_eq!(proj_enabled, Some(true), "Project settings should be true");
        assert_eq!(local_enabled, Some(false), "Local settings should be false");

        // The effective state should be DISABLED because local=false wins
        // This is tested by creating a mock plugin and checking is_enabled()
        let plugin = Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::Local,
            install_path: None,
            project_path: Some(project_a.path().to_path_buf()),
            is_current_project: false, // Installed in different project
            enabled_user: None,
            enabled_project: proj_enabled,
            enabled_local: local_enabled,
            installed_at: None,
            last_updated: None,
        };

        // Local false MUST override Project true
        assert!(
            !plugin.is_enabled(),
            "Plugin should be DISABLED because local=false overrides project=true"
        );
    }

    #[test]
    fn test_precedence_local_false_overrides_project_true() {
        // This test verifies the precedence fix from earlier is working
        let plugin = Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::Local,
            install_path: None,
            project_path: None,
            is_current_project: true,
            enabled_user: None,
            enabled_project: Some(true),  // Project says enabled
            enabled_local: Some(false),   // Local says disabled
            installed_at: None,
            last_updated: None,
        };

        // Local false MUST override Project true
        assert!(
            !plugin.is_enabled(),
            "Local false should override Project true"
        );
    }

    #[test]
    fn test_enabled_state_reads_from_plugin_project_path() {
        // Verify ConfigPaths::load_settings_from_project works correctly
        let temp = TempDir::new().unwrap();

        // Create settings with specific plugin enabled
        create_test_settings(temp.path(), &[("my-plugin@marketplace", true)]);
        create_test_local_settings(temp.path(), &[("my-plugin@marketplace", false)]);

        let (proj_settings, local_settings) = ConfigPaths::load_settings_from_project(temp.path());

        assert!(proj_settings.is_some(), "Project settings should be loaded");
        assert!(local_settings.is_some(), "Local settings should be loaded");

        let proj_enabled = proj_settings
            .unwrap()
            .enabled_plugins
            .get("my-plugin@marketplace")
            .copied();
        let local_enabled = local_settings
            .unwrap()
            .enabled_plugins
            .get("my-plugin@marketplace")
            .copied();

        assert_eq!(proj_enabled, Some(true));
        assert_eq!(local_enabled, Some(false));
    }

    #[test]
    fn test_user_scope_ignores_project_settings() {
        // User scope plugins should not have project/local settings applied
        // (they are global, not tied to any project)
        let plugin = Plugin {
            id: "test@marketplace".to_string(),
            name: "test".to_string(),
            marketplace: "marketplace".to_string(),
            description: None,
            version: None,
            author: None,
            install_scope: Scope::User, // User scope!
            install_path: None,
            project_path: None,
            is_current_project: true,
            enabled_user: Some(true),  // Only user setting matters
            enabled_project: None,     // Should be None for user-scope plugins
            enabled_local: None,       // Should be None for user-scope plugins
            installed_at: None,
            last_updated: None,
        };

        assert!(
            plugin.is_enabled(),
            "User-scope plugin enabled in user settings should be enabled"
        );
        assert_eq!(
            plugin.effective_scope(),
            Some("User"),
            "Effective scope should be User"
        );
    }

    #[test]
    fn test_load_settings_from_nonexistent_project() {
        // Should return (None, None) for non-existent path
        let temp = TempDir::new().unwrap();
        let fake_path = temp.path().join("does-not-exist");

        let (proj_settings, local_settings) = ConfigPaths::load_settings_from_project(&fake_path);

        assert!(proj_settings.is_none(), "Should be None for non-existent project");
        assert!(local_settings.is_none(), "Should be None for non-existent local");
    }
}
