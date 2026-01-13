use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

use super::alerts::engine::AlertEngineConfig;

/// Application settings with alert configuration.
/// NOTE: TypeScript mirror types are in ui/src/types.ts
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub gamelog_dir: PathBuf,
    pub dps_window_seconds: u64,
    /// Alert system configuration
    #[serde(default)]
    pub alert_settings: AlertEngineConfig,
}

impl Default for Settings {
    fn default() -> Self {
        // Try to guess the default EVE log path, or fallback to something safe
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        
        // This is a rough default, the user will likely change it.
        let default_path = PathBuf::from(home)
            .join("Documents/EVE/logs/Gamelogs");
            
        Self {
            gamelog_dir: default_path,
            dps_window_seconds: 5,
            alert_settings: AlertEngineConfig::default_enabled(),
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new(app_config_dir: PathBuf) -> Self {
        Self {
            config_path: app_config_dir.join("settings.json"),
        }
    }

    pub fn load(&self) -> Settings {
        if self.config_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.config_path) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    return settings;
                }
            }
        }
        Settings::default()
    }

    pub fn save(&self, settings: &Settings) -> io::Result<()> {
        // Ensure directory exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(&self.config_path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let manager = ConfigManager::new(dir.path().to_path_buf());

        let default = manager.load();
        assert_eq!(default.dps_window_seconds, 5);

        let new_settings = Settings {
            gamelog_dir: PathBuf::from("/tmp/logs"),
            dps_window_seconds: 10,
            alert_settings: AlertEngineConfig::default_enabled(),
        };

        manager.save(&new_settings).unwrap();
        let loaded = manager.load();
        
        assert_eq!(loaded.gamelog_dir, PathBuf::from("/tmp/logs"));
        assert_eq!(loaded.dps_window_seconds, 10);
    }
}
