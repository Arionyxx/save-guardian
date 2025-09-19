use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SaveType {
    Steam,
    NonSteam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamUser {
    pub id: String,
    pub name: Option<String>,
    pub path: PathBuf,
    pub games: Vec<GameSave>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSave {
    pub name: String,
    pub app_id: Option<u32>, // Steam app ID, None for non-Steam games
    pub save_type: SaveType,
    pub save_path: PathBuf,
    pub last_modified: Option<DateTime<Utc>>,
    pub size: u64,
    pub backup_count: usize,
    pub is_synced: bool, // Whether this save has a corresponding Steam/non-Steam version
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveLocation {
    pub path: PathBuf,
    pub location_type: LocationType,
    pub description: String,
    pub is_custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationType {
    Documents,
    AppDataRoaming,
    AppDataLocal,
    AppDataLocalLow,
    PublicDocuments,
    GameInstall,
    Steam,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub game_name: String,
    pub app_id: Option<u32>,
    pub save_type: SaveType,
    pub original_path: PathBuf,
    pub backup_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub size: u64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPair {
    pub steam_save: Option<GameSave>,
    pub non_steam_save: Option<GameSave>,
    pub game_name: String,
    pub app_id: Option<u32>,
    pub last_synced: Option<DateTime<Utc>>,
    pub sync_direction: SyncDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncDirection {
    SteamToNonSteam,
    NonSteamToSteam,
    Bidirectional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub steam_path: PathBuf,
    pub backup_path: PathBuf,
    pub custom_locations: Vec<SaveLocation>,
    pub auto_backup: bool,
    pub backup_retention_days: u32,
    pub theme: Theme,
    pub window_size: (f32, f32),
    pub window_position: Option<(f32, f32)>,
    pub koofr_config: KoofrConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoofrConfig {
    pub enabled: bool,
    pub server_url: String,
    pub username: String,
    pub password: String, // In a real app, this should be encrypted
    pub sync_folder: String,
    pub auto_sync: bool,
    pub sync_interval_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            steam_path: PathBuf::from(r"C:\Program Files (x86)\Steam\userdata"),
            backup_path: dirs::document_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("SaveGuardianBackups"),
            custom_locations: Vec::new(),
            auto_backup: true,
            backup_retention_days: 30,
            theme: Theme::Dark,
            window_size: (1200.0, 800.0),
            window_position: None,
            koofr_config: KoofrConfig::default(),
        }
    }
}

impl Default for KoofrConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: "https://app.koofr.net/dav/Koofr".to_string(),
            username: String::new(),
            password: String::new(),
            sync_folder: "/SaveGuardian".to_string(),
            auto_sync: false,
            sync_interval_minutes: 30,
        }
    }
}

impl BackupInfo {
    /// Get a display name for the original path
    pub fn display_original_path(&self) -> String {
        let path_str = self.original_path.to_string_lossy();
        
        // Check if this is a downloaded backup
        if path_str.contains("Downloaded from cloud") || path_str.contains("cloud") {
            // Try to extract the original path from description or use a better fallback
            if let Some(ref desc) = self.description {
                if desc.contains("Downloaded from cloud") {
                    return "📥 Downloaded from Cloud Storage".to_string();
                }
            }
            return "📥 Cloud Download".to_string();
        }
        
        // For regular backups, show the full original path
        path_str.to_string()
    }
    
    /// Get a formatted size string
    pub fn format_size(&self) -> String {
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else if self.size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", self.size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
    
    /// Check if this backup was downloaded from cloud
    pub fn is_cloud_download(&self) -> bool {
        let path_str = self.original_path.to_string_lossy();
        path_str.contains("Downloaded from cloud") || path_str.contains("cloud") ||
        self.description.as_ref().map_or(false, |d| d.contains("Downloaded from cloud"))
    }
}

impl GameSave {
    pub fn new(name: String, path: PathBuf, save_type: SaveType, app_id: Option<u32>) -> Self {
        let metadata = std::fs::metadata(&path).ok();
        let last_modified = metadata.as_ref().and_then(|m| {
            m.modified()
                .ok()
                .map(|t| DateTime::<Utc>::from(t))
        });
        let size = metadata.map(|m| m.len()).unwrap_or(0);

        Self {
            name,
            app_id,
            save_type,
            save_path: path,
            last_modified,
            size,
            backup_count: 0,
            is_synced: false,
        }
    }

    pub fn format_size(&self) -> String {
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else if self.size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", self.size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    pub fn display_name(&self) -> String {
        match &self.app_id {
            Some(id) => format!("{} ({})", self.name, id),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SaveGuardianError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),
    
    #[error("Invalid Steam user ID: {0}")]
    InvalidSteamUser(String),
    
    #[error("Save operation failed: {0}")]
    SaveOperationFailed(String),
    
    #[error("Backup operation failed: {0}")]
    BackupOperationFailed(String),
}

pub type Result<T> = std::result::Result<T, SaveGuardianError>;