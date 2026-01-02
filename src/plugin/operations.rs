use super::{
    config::{ConfigPaths, KnownMarketplaces, Settings},
    Plugin, PluginError, Result, Scope,
};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Metadata stored in lock files for debugging and stale detection
#[derive(Debug, Serialize, Deserialize)]
struct LockMetadata {
    pid: u32,
    timestamp: DateTime<Utc>,
}

/// Guard that holds a lock file and ensures it is deleted when dropped.
/// This mimics vim's swap file behavior - the lock file is automatically
/// cleaned up when the guard goes out of scope (normal completion or panic).
#[derive(Debug)]
pub struct LockFileGuard {
    lock_path: PathBuf,
    _file: File, // Holds the exclusive lock
}

impl Drop for LockFileGuard {
    fn drop(&mut self) {
        // Best effort removal - ignore errors (file might already be gone)
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Check if a process with the given PID is currently running.
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    // On Unix, sending signal 0 checks if process exists without sending a signal
    // Returns 0 if process exists (regardless of permissions), -1 with ESRCH if not
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // On non-Unix platforms, conservatively assume the process might be running
    // to avoid accidentally overwriting active locks
    true
}

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

    fn acquire_lock(&self, path: &Path) -> Result<LockFileGuard> {
        let lock_path = path.with_extension("lock");

        // Ensure parent directory exists
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        // Check for existing lock file and handle stale locks
        if lock_path.exists() {
            if let Ok(content) = fs::read_to_string(&lock_path) {
                if let Ok(metadata) = serde_json::from_str::<LockMetadata>(&content) {
                    if is_process_running(metadata.pid) {
                        // Lock is held by an active process
                        return Err(PluginError::LockConflict {
                            path: lock_path,
                            pid: metadata.pid,
                        });
                    }
                    // Process is dead - lock is stale, remove it
                    let _ = fs::remove_file(&lock_path);
                }
                // If we can't parse the metadata, treat as stale and overwrite
            }
            // If we can't read the file, try to overwrite it
        }

        // Create lock file with exclusive access
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .map_err(|source| PluginError::LockError {
                path: lock_path.clone(),
                source,
            })?;

        // Try to acquire exclusive lock (non-blocking)
        file.try_lock_exclusive()
            .map_err(|source| PluginError::LockError {
                path: lock_path.clone(),
                source,
            })?;

        // Write metadata to lock file
        let metadata = LockMetadata {
            pid: std::process::id(),
            timestamp: Utc::now(),
        };
        let metadata_json = serde_json::to_string(&metadata).map_err(|e| {
            PluginError::ConfigParseError {
                path: lock_path.clone(),
                source: e,
            }
        })?;

        // Write metadata - use a separate file handle to avoid moving the locked file
        fs::write(&lock_path, metadata_json).map_err(|source| PluginError::ConfigWriteError {
            path: lock_path.clone(),
            source,
        })?;

        Ok(LockFileGuard {
            lock_path,
            _file: file,
        })
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

    #[test]
    fn test_lock_file_contains_pid_and_timestamp() {
        let (_temp, service) = setup_test_env();

        let settings_path = service.paths.user_settings();
        let lock_path = settings_path.with_extension("lock");

        // Acquire lock - this creates the lock file with metadata
        let guard = service.acquire_lock(&settings_path).unwrap();

        // Read lock file and verify metadata
        let content = fs::read_to_string(&lock_path).unwrap();
        let metadata: LockMetadata = serde_json::from_str(&content).unwrap();

        assert_eq!(metadata.pid, std::process::id());
        // Timestamp should be recent (within last minute)
        let now = Utc::now();
        let diff = now.signed_duration_since(metadata.timestamp);
        assert!(diff.num_seconds() < 60);

        // Lock file exists while guard is alive
        assert!(lock_path.exists());

        drop(guard);
    }

    #[test]
    fn test_lock_file_deleted_on_drop() {
        let (_temp, service) = setup_test_env();

        let settings_path = service.paths.user_settings();
        let lock_path = settings_path.with_extension("lock");

        {
            let _guard = service.acquire_lock(&settings_path).unwrap();
            assert!(lock_path.exists(), "Lock file should exist while guard is alive");
        }
        // Guard dropped here

        assert!(!lock_path.exists(), "Lock file should be deleted on drop");
    }

    #[test]
    fn test_stale_lock_detection_dead_process() {
        let (_temp, service) = setup_test_env();

        let settings_path = service.paths.user_settings();
        let lock_path = settings_path.with_extension("lock");

        // Write a lock file with a non-existent PID (99999999 should not exist)
        let stale_metadata = LockMetadata {
            pid: 99999999,
            timestamp: Utc::now(),
        };
        fs::write(&lock_path, serde_json::to_string(&stale_metadata).unwrap()).unwrap();

        // Should succeed because the PID is dead (stale lock)
        let guard = service.acquire_lock(&settings_path);
        assert!(guard.is_ok(), "Should acquire lock when existing lock is stale");

        // Verify the new lock has our PID
        let content = fs::read_to_string(&lock_path).unwrap();
        let metadata: LockMetadata = serde_json::from_str(&content).unwrap();
        assert_eq!(metadata.pid, std::process::id());

        drop(guard);
    }

    #[test]
    fn test_lock_conflict_returns_error() {
        let (_temp, service) = setup_test_env();

        let settings_path = service.paths.user_settings();
        let lock_path = settings_path.with_extension("lock");

        // Write a lock file with our own PID (which is definitely running)
        let active_metadata = LockMetadata {
            pid: std::process::id(),
            timestamp: Utc::now(),
        };
        fs::write(&lock_path, serde_json::to_string(&active_metadata).unwrap()).unwrap();

        // Should fail with LockConflict because the PID is active
        let result = service.acquire_lock(&settings_path);

        match result {
            Err(PluginError::LockConflict { pid, path: _ }) => {
                assert_eq!(pid, std::process::id());
            }
            _ => panic!("Expected LockConflict error, got {:?}", result),
        }

        // Clean up the lock file manually since no guard was created
        fs::remove_file(&lock_path).ok();
    }
}
