use crate::types::{Config, Result, SaveGuardianError};
use std::fs;
use std::path::PathBuf;

impl Config {
    /// Load configuration from file
    pub fn load_from_file(path: &PathBuf) -> Result<Config> {
        if !path.exists() {
            return Ok(Config::default());
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| SaveGuardianError::Io(e))?;
        
        let config: Config = toml::from_str(&contents)
            .map_err(|e| SaveGuardianError::Toml(e))?;
        
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|_| SaveGuardianError::SaveOperationFailed("Failed to serialize config".to_string()))?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SaveGuardianError::Io(e))?;
        }
        
        fs::write(path, contents)
            .map_err(|e| SaveGuardianError::Io(e))?;
        
        Ok(())
    }

    /// Get the default config file path
    pub fn get_config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("save-guardian").join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }
}