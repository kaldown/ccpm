use super::{
    config::{ConfigPaths, KnownMarketplaces, Settings},
    Plugin, PluginError, Result, Scope,
};
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub struct PluginService {
    paths: ConfigPaths,
}

impl PluginService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            paths: ConfigPaths::new()?,
        })
    }

    pub fn with_paths(paths: ConfigPaths) -> Self {
        Self { paths }
    }

    /// Enable a plugin in the specified scope
    pub fn enable_plugin(&self, id: &str, scope: Scope) -> Result<()> {
        self.set_plugin_enabled(id, scope, true)
    }

    /// Disable a plugin in the specified scope
    pub fn disable_plugin(&self, id: &str, scope: Scope) -> Result<()> {
        self.set_plugin_enabled(id, scope, false)
    }

    /// Toggle plugin enabled state
    pub fn toggle_plugin(&self, plugin: &Plugin) -> Result<bool> {
        let new_state = !plugin.is_enabled();
        self.set_plugin_enabled(&plugin.id, plugin.install_scope, new_state)?;
        Ok(new_state)
    }

    /// Toggle auto-update for a marketplace
    pub fn toggle_auto_update(&self, marketplace: &str) -> Result<bool> {
        let path = self.paths.known_marketplaces();
        let _lock = self.acquire_lock(&path)?;

        let mut marketplaces = self.load_known_marketplaces();

        let entry = marketplaces
            .marketplaces
            .get_mut(marketplace)
            .ok_or_else(|| PluginError::MarketplaceNotFound(marketplace.to_string()))?;

        entry.auto_update = !entry.auto_update;
        let new_state = entry.auto_update;

        self.write_json_atomic(&path, &marketplaces)?;

        Ok(new_state)
    }

    /// Get auto-update status for a marketplace
    pub fn get_auto_update(&self, marketplace: &str) -> Result<bool> {
        let marketplaces = self.load_known_marketplaces();
        marketplaces
            .marketplaces
            .get(marketplace)
            .map(|e| e.auto_update)
            .ok_or_else(|| PluginError::MarketplaceNotFound(marketplace.to_string()))
    }

    fn set_plugin_enabled(&self, id: &str, scope: Scope, enabled: bool) -> Result<()> {
        let path = match scope {
            Scope::User => self.paths.user_settings(),
            Scope::Project => self.paths.project_settings(),
            Scope::Local => self.paths.local_settings(),
        };

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| PluginError::ConfigWriteError {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let _lock = self.acquire_lock(&path)?;

        let mut settings = self.load_settings(&path);
        settings.enabled_plugins.insert(id.to_string(), enabled);

        self.write_json_atomic(&path, &settings)?;

        Ok(())
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

    fn acquire_lock(&self, path: &Path) -> Result<File> {
        let lock_path = path.with_extension("lock");

        // Ensure parent directory exists
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .map_err(|source| PluginError::LockError {
                path: lock_path.clone(),
                source,
            })?;

        file.try_lock_exclusive()
            .map_err(|source| PluginError::LockError {
                path: lock_path,
                source,
            })?;

        Ok(file)
    }

    fn write_json_atomic<T: serde::Serialize>(&self, path: &Path, data: &T) -> Result<()> {
        let temp_path = path.with_extension("tmp");

        // Write to temp file
        let json =
            serde_json::to_string_pretty(data).map_err(|e| PluginError::ConfigParseError {
                path: path.to_path_buf(),
                source: e,
            })?;

        let mut file =
            File::create(&temp_path).map_err(|source| PluginError::ConfigWriteError {
                path: temp_path.clone(),
                source,
            })?;

        file.write_all(json.as_bytes())
            .map_err(|source| PluginError::ConfigWriteError {
                path: temp_path.clone(),
                source,
            })?;

        file.sync_all()
            .map_err(|source| PluginError::ConfigWriteError {
                path: temp_path.clone(),
                source,
            })?;

        // Atomic rename
        fs::rename(&temp_path, path).map_err(|source| PluginError::ConfigWriteError {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, PluginService) {
        let temp = TempDir::new().unwrap();
        let paths = ConfigPaths {
            user_dir: temp.path().join("user"),
            local_dir: temp.path().join("local"),
        };
        fs::create_dir_all(&paths.user_dir).unwrap();
        fs::create_dir_all(&paths.local_dir).unwrap();

        let service = PluginService::with_paths(paths);
        (temp, service)
    }

    #[test]
    fn test_enable_disable_plugin() {
        let (_temp, service) = setup_test_env();

        service
            .enable_plugin("test@marketplace", Scope::User)
            .unwrap();

        let settings: Settings =
            serde_json::from_str(&fs::read_to_string(service.paths.user_settings()).unwrap())
                .unwrap();

        assert_eq!(
            settings.enabled_plugins.get("test@marketplace"),
            Some(&true)
        );

        service
            .disable_plugin("test@marketplace", Scope::User)
            .unwrap();

        let settings: Settings =
            serde_json::from_str(&fs::read_to_string(service.paths.user_settings()).unwrap())
                .unwrap();

        assert_eq!(
            settings.enabled_plugins.get("test@marketplace"),
            Some(&false)
        );
    }
}
