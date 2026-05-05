use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// Claude Code settings.json structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub enabled_plugins: BTreeMap<String, bool>,

    #[serde(flatten)]
    pub other: BTreeMap<String, serde_json::Value>,
}

/// Installed plugins tracking file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugins {
    pub version: u32,
    pub plugins: HashMap<String, Vec<InstalledPluginEntry>>,
}

impl Default for InstalledPlugins {
    fn default() -> Self {
        Self {
            version: 2,
            plugins: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPluginEntry {
    pub scope: String,
    pub install_path: PathBuf,
    #[serde(default)]
    pub project_path: Option<PathBuf>, // For project/local scopes - the project directory
    pub version: String,
    pub installed_at: String,
    pub last_updated: String,
    #[serde(default)]
    pub git_commit_sha: Option<String>,
    #[serde(default)]
    pub is_local: bool,
}

/// Known marketplaces tracking file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnownMarketplaces {
    #[serde(flatten)]
    pub marketplaces: HashMap<String, MarketplaceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceEntry {
    pub source: MarketplaceSource,
    pub install_location: PathBuf,
    pub last_updated: String,
    #[serde(default)]
    pub auto_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSource {
    pub source: String,
    pub repo: String,
}

/// Plugin manifest file structure (plugin.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<PluginAuthor>,
    #[serde(default)]
    pub mcp_servers: Option<HashMap<String, McpServer>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Paths for Claude Code configuration files
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub user_dir: PathBuf,
    pub local_dir: PathBuf,
}

impl ConfigPaths {
    pub fn new() -> super::Result<Self> {
        let home = dirs::home_dir().ok_or(super::PluginError::HomeDirNotFound)?;
        let user_dir = home.join(".claude");
        let local_dir = PathBuf::from(".claude");

        Ok(Self {
            user_dir,
            local_dir,
        })
    }

    pub fn user_settings(&self) -> PathBuf {
        self.user_dir.join("settings.json")
    }

    /// Project-scope settings (./.claude/settings.json) - shared in git
    pub fn project_settings(&self) -> PathBuf {
        self.local_dir.join("settings.json")
    }

    /// Local-scope settings (./.claude/settings.local.json) - gitignored
    pub fn local_settings(&self) -> PathBuf {
        self.local_dir.join("settings.local.json")
    }

    pub fn installed_plugins(&self) -> PathBuf {
        self.user_dir.join("plugins").join("installed_plugins.json")
    }

    pub fn known_marketplaces(&self) -> PathBuf {
        self.user_dir
            .join("plugins")
            .join("known_marketplaces.json")
    }

    pub fn plugin_cache(&self) -> PathBuf {
        self.user_dir.join("plugins").join("cache")
    }

    pub fn marketplaces(&self) -> PathBuf {
        self.user_dir.join("plugins").join("marketplaces")
    }
}

impl ConfigPaths {
    /// Load settings from a specific project directory (not CWD).
    /// Used to read settings from the plugin's actual project, not the current working directory.
    pub fn load_settings_from_project(
        project_path: &std::path::Path,
    ) -> (Option<Settings>, Option<Settings>) {
        let claude_dir = project_path.join(".claude");

        let project_settings = Self::load_settings_file(&claude_dir.join("settings.json"));
        let local_settings = Self::load_settings_file(&claude_dir.join("settings.local.json"));

        (project_settings, local_settings)
    }

    fn load_settings_file(path: &std::path::Path) -> Option<Settings> {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
        } else {
            None
        }
    }
}

impl Default for ConfigPaths {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            user_dir: PathBuf::from(".claude"),
            local_dir: PathBuf::from(".claude"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert!(settings.enabled_plugins.is_empty());
        assert!(settings.other.is_empty());
    }

    #[test]
    fn test_settings_deserialize() {
        let json = r#"{
            "enabledPlugins": {
                "test@marketplace": true,
                "other@marketplace": false
            },
            "someOtherField": "value"
        }"#;

        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(
            settings.enabled_plugins.get("test@marketplace"),
            Some(&true)
        );
        assert_eq!(
            settings.enabled_plugins.get("other@marketplace"),
            Some(&false)
        );
        assert!(settings.other.contains_key("someOtherField"));
    }

    #[test]
    fn test_installed_plugins_default() {
        let installed = InstalledPlugins::default();
        assert_eq!(installed.version, 2);
        assert!(installed.plugins.is_empty());
    }

    #[test]
    fn test_config_paths() {
        let paths = ConfigPaths::new().unwrap();
        assert!(paths.user_settings().ends_with("settings.json"));
        assert!(paths.project_settings().ends_with("settings.json"));
        assert!(paths.local_settings().ends_with("settings.local.json"));
        assert!(paths
            .installed_plugins()
            .to_string_lossy()
            .contains("installed_plugins.json"));
        assert!(paths
            .known_marketplaces()
            .to_string_lossy()
            .contains("known_marketplaces.json"));
    }

    #[test]
    fn test_plugin_manifest_deserialize() {
        let json = r#"{
            "name": "test-plugin",
            "description": "A test plugin",
            "version": "1.0.0",
            "author": {
                "name": "Test Author",
                "email": "test@example.com"
            }
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.description, Some("A test plugin".to_string()));
        assert_eq!(manifest.version, Some("1.0.0".to_string()));
        assert!(manifest.author.is_some());
        let author = manifest.author.unwrap();
        assert_eq!(author.name, "Test Author");
        assert_eq!(author.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_known_marketplaces_deserialize() {
        let json = r#"{
            "test-marketplace": {
                "source": {
                    "source": "github",
                    "repo": "owner/repo"
                },
                "installLocation": "/path/to/install",
                "lastUpdated": "2025-01-01T00:00:00Z",
                "autoUpdate": true
            }
        }"#;

        let marketplaces: KnownMarketplaces = serde_json::from_str(json).unwrap();
        let entry = marketplaces.marketplaces.get("test-marketplace").unwrap();
        assert_eq!(entry.source.source, "github");
        assert_eq!(entry.source.repo, "owner/repo");
        assert!(entry.auto_update);
    }

    #[test]
    fn test_settings_serialize_keys_sorted() {
        // Insert in non-alphabetical order so HashMap (random) would almost
        // certainly NOT produce alphabetical output by accident.
        let mut settings = Settings::default();
        for id in &[
            "zebra@m",
            "alpha@m",
            "mango@m",
            "kiwi@m",
            "papaya@m",
            "banana@m",
        ] {
            settings.enabled_plugins.insert((*id).to_string(), true);
        }

        let json = serde_json::to_string_pretty(&settings).unwrap();

        // Find the line indices of each plugin id in the serialized output.
        let positions: Vec<(&str, usize)> = [
            "alpha@m", "banana@m", "kiwi@m", "mango@m", "papaya@m", "zebra@m",
        ]
        .iter()
        .map(|id| (*id, json.find(id).unwrap_or_else(|| panic!("missing {id}"))))
        .collect();

        // Each id must appear at a strictly increasing byte offset, i.e. alphabetical order.
        for window in positions.windows(2) {
            assert!(
                window[0].1 < window[1].1,
                "expected {} before {} in serialized JSON, got positions {} and {}",
                window[0].0,
                window[1].0,
                window[0].1,
                window[1].1,
            );
        }
    }
}
