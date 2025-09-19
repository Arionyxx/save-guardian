use crate::types::*;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use walkdir::WalkDir;
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};
use chrono::Utc;
use log::{debug, info, warn};
use serde::{Serialize, Deserialize};

pub struct BackupManager {
    backup_root: PathBuf,
    retention_days: u32,
}

impl BackupManager {
    pub fn new(backup_root: PathBuf, retention_days: u32) -> Result<Self> {
        // Create backup directory if it doesn't exist
        if !backup_root.exists() {
            fs::create_dir_all(&backup_root)
                .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to create backup directory: {}", e)))?;
            info!("Created backup directory: {:?}", backup_root);
        }

        Ok(Self {
            backup_root,
            retention_days,
        })
    }

    /// Create a backup of a game save
    pub fn create_backup(&self, game_save: &GameSave, description: Option<String>) -> Result<BackupInfo> {
        let backup_id = self.generate_backup_id(game_save);
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_filename = format!("{}_{}.zip", backup_id, timestamp);
        let backup_path = self.backup_root.join(&backup_filename);

        info!("Creating backup for {} at {:?}", game_save.name, backup_path);

        // Create the ZIP backup
        let backup_size = self.create_zip_backup(&game_save.save_path, &backup_path)?;

        let backup_info = BackupInfo {
            id: backup_id,
            game_name: game_save.name.clone(),
            app_id: game_save.app_id,
            save_type: game_save.save_type.clone(),
            original_path: game_save.save_path.clone(),
            backup_path,
            created_at: Utc::now(),
            size: backup_size,
            description,
        };

        // Save backup metadata
        self.save_backup_metadata(&backup_info)?;

        info!("Backup created successfully: {}", backup_info.id);
        Ok(backup_info)
    }

    /// Create a ZIP backup of a directory or file
    fn create_zip_backup(&self, source_path: &PathBuf, backup_path: &PathBuf) -> Result<u64> {
        let backup_file = fs::File::create(backup_path)
            .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to create backup file: {}", e)))?;

        let mut zip = ZipWriter::new(backup_file);
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755);

        if source_path.is_file() {
            // Backup single file
            let mut file = fs::File::open(source_path)
                .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to open source file: {}", e)))?;
            
            let filename = source_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            
            zip.start_file(filename, options)
                .map_err(|e| SaveGuardianError::Zip(e))?;
            
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| SaveGuardianError::Io(e))?;
            
            zip.write_all(&buffer)
                .map_err(|e| SaveGuardianError::Io(e))?;
        } else if source_path.is_dir() {
            // Backup directory
            let walker = WalkDir::new(source_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok());

            for entry in walker {
                let path = entry.path();
                let relative_path = path.strip_prefix(source_path)
                    .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Path error: {}", e)))?;

                if path.is_file() {
                    let mut file = fs::File::open(path)
                        .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to open file: {}", e)))?;

                    let file_path_str = relative_path.to_string_lossy().replace('\\', "/");
                    zip.start_file(&file_path_str, options)
                        .map_err(|e| SaveGuardianError::Zip(e))?;

                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer)
                        .map_err(|e| SaveGuardianError::Io(e))?;

                    zip.write_all(&buffer)
                        .map_err(|e| SaveGuardianError::Io(e))?;

                    debug!("Added file to backup: {}", file_path_str);
                } else if path.is_dir() && relative_path.as_os_str() != "" {
                    // Add directory entry
                    let dir_path_str = format!("{}/", relative_path.to_string_lossy().replace('\\', "/"));
                    zip.add_directory(&dir_path_str, options)
                        .map_err(|e| SaveGuardianError::Zip(e))?;

                    debug!("Added directory to backup: {}", dir_path_str);
                }
            }
        } else {
            return Err(SaveGuardianError::BackupOperationFailed(
                "Source path is neither file nor directory".to_string()
            ));
        }

        let zip_file = zip.finish()
            .map_err(|e| SaveGuardianError::Zip(e))?;

        let backup_size = zip_file.metadata()
            .map_err(|e| SaveGuardianError::Io(e))?
            .len();

        Ok(backup_size)
    }

    /// Restore a backup to a specified location
    pub fn restore_backup(&self, backup_info: &BackupInfo, restore_path: &PathBuf, overwrite: bool) -> Result<()> {
        info!("Restoring backup {} to {:?}", backup_info.id, restore_path);

        if restore_path.exists() && !overwrite {
            return Err(SaveGuardianError::BackupOperationFailed(
                "Restore path already exists and overwrite is disabled".to_string()
            ));
        }

        // Create parent directories if they don't exist
        if let Some(parent) = restore_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to create restore directory: {}", e)))?;
        }

        // Extract the ZIP backup
        self.extract_zip_backup(&backup_info.backup_path, restore_path)?;

        info!("Backup restored successfully to {:?}", restore_path);
        Ok(())
    }

    /// Extract a ZIP backup to a directory
    fn extract_zip_backup(&self, zip_path: &PathBuf, extract_path: &PathBuf) -> Result<()> {
        let zip_file = fs::File::open(zip_path)
            .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to open backup file: {}", e)))?;

        let mut archive = ZipArchive::new(zip_file)
            .map_err(|e| SaveGuardianError::Zip(e))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| SaveGuardianError::Zip(e))?;

            let file_path = extract_path.join(file.name());

            if file.name().ends_with('/') {
                // Directory
                fs::create_dir_all(&file_path)
                    .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to create directory: {}", e)))?;
            } else {
                // File
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to create parent directory: {}", e)))?;
                }

                let mut output_file = fs::File::create(&file_path)
                    .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to create output file: {}", e)))?;

                std::io::copy(&mut file, &mut output_file)
                    .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to extract file: {}", e)))?;

                debug!("Extracted file: {:?}", file_path);
            }
        }

        Ok(())
    }

    /// List all backups for a specific game
    pub fn list_backups(&self, game_name: Option<&str>, app_id: Option<u32>) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();

        // Read backup metadata files
        let metadata_pattern = "*.backup.json";
        let entries = fs::read_dir(&self.backup_root)
            .map_err(|e| SaveGuardianError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| SaveGuardianError::Io(e))?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    if filename.ends_with(".backup") {
                        if let Ok(backup_info) = self.load_backup_metadata(&path) {
                            // Filter by game name or app ID if specified
                            let matches = match (game_name, app_id) {
                                (Some(name), Some(id)) => backup_info.game_name.contains(name) && backup_info.app_id == Some(id),
                                (Some(name), None) => backup_info.game_name.contains(name),
                                (None, Some(id)) => backup_info.app_id == Some(id),
                                (None, None) => true,
                            };

                            if matches {
                                backups.push(backup_info);
                            }
                        }
                    }
                }
            }
        }

        // Sort by creation date (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Delete a backup
    pub fn delete_backup(&self, backup_info: &BackupInfo) -> Result<()> {
        info!("Deleting backup: {}", backup_info.id);

        // Delete the backup file
        if backup_info.backup_path.exists() {
            fs::remove_file(&backup_info.backup_path)
                .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to delete backup file: {}", e)))?;
        }

        // Delete the metadata file
        let metadata_path = self.get_metadata_path(&backup_info.id);
        if metadata_path.exists() {
            fs::remove_file(&metadata_path)
                .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to delete metadata file: {}", e)))?;
        }

        info!("Backup deleted successfully: {}", backup_info.id);
        Ok(())
    }

    /// Clean up old backups based on retention policy
    pub fn cleanup_old_backups(&self) -> Result<usize> {
        let cutoff_date = Utc::now() - chrono::Duration::days(self.retention_days as i64);
        let all_backups = self.list_backups(None, None)?;

        let mut deleted_count = 0;
        for backup in all_backups {
            if backup.created_at < cutoff_date {
                match self.delete_backup(&backup) {
                    Ok(_) => {
                        deleted_count += 1;
                        info!("Deleted old backup: {}", backup.id);
                    }
                    Err(e) => {
                        warn!("Failed to delete old backup {}: {}", backup.id, e);
                    }
                }
            }
        }

        if deleted_count > 0 {
            info!("Cleaned up {} old backups", deleted_count);
        }

        Ok(deleted_count)
    }

    /// Generate a unique backup ID
    fn generate_backup_id(&self, game_save: &GameSave) -> String {
        let game_name_clean = game_save.name.replace(' ', "_").replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let app_id_part = match game_save.app_id {
            Some(id) => format!("_{}", id),
            None => String::new(),
        };
        let save_type = match game_save.save_type {
            SaveType::Steam => "steam",
            SaveType::NonSteam => "nonsteam",
        };

        format!("{}{}_{}", game_name_clean, app_id_part, save_type)
    }

    /// Save backup metadata to a JSON file
    fn save_backup_metadata(&self, backup_info: &BackupInfo) -> Result<()> {
        let metadata_path = self.get_metadata_path(&backup_info.id);
        let metadata_json = serde_json::to_string_pretty(backup_info)
            .map_err(|e| SaveGuardianError::Serde(e))?;

        fs::write(&metadata_path, metadata_json)
            .map_err(|e| SaveGuardianError::BackupOperationFailed(format!("Failed to save metadata: {}", e)))?;

        debug!("Saved backup metadata: {:?}", metadata_path);
        Ok(())
    }

    /// Load backup metadata from a JSON file
    fn load_backup_metadata(&self, metadata_path: &PathBuf) -> Result<BackupInfo> {
        let metadata_json = fs::read_to_string(metadata_path)
            .map_err(|e| SaveGuardianError::Io(e))?;

        let backup_info: BackupInfo = serde_json::from_str(&metadata_json)
            .map_err(|e| SaveGuardianError::Serde(e))?;

        Ok(backup_info)
    }

    /// Get the metadata file path for a backup ID
    fn get_metadata_path(&self, backup_id: &str) -> PathBuf {
        self.backup_root.join(format!("{}.backup.json", backup_id))
    }

    /// Get backup statistics
    pub fn get_backup_stats(&self) -> Result<BackupStats> {
        let all_backups = self.list_backups(None, None)?;
        let total_count = all_backups.len();
        let total_size = all_backups.iter().map(|b| b.size).sum();

        let mut steam_count = 0;
        let mut non_steam_count = 0;
        let mut oldest_backup = None;
        let mut newest_backup = None;

        for backup in &all_backups {
            match backup.save_type {
                SaveType::Steam => steam_count += 1,
                SaveType::NonSteam => non_steam_count += 1,
            }

            if oldest_backup.is_none() || backup.created_at < oldest_backup.unwrap() {
                oldest_backup = Some(backup.created_at);
            }

            if newest_backup.is_none() || backup.created_at > newest_backup.unwrap() {
                newest_backup = Some(backup.created_at);
            }
        }

        Ok(BackupStats {
            total_count,
            total_size,
            steam_count,
            non_steam_count,
            oldest_backup,
            newest_backup,
        })
    }
    
    /// Open the backup folder in the system file explorer
    pub fn open_backup_folder(&self, backup_info: &BackupInfo) -> Result<()> {
        let folder_path = if backup_info.backup_path.is_file() {
            backup_info.backup_path.parent().unwrap_or(&self.backup_root)
        } else {
            &backup_info.backup_path
        };
        
        #[cfg(windows)]
        {
            std::process::Command::new("explorer")
                .arg("/select,")
                .arg(&backup_info.backup_path)
                .spawn()
                .map_err(|e| SaveGuardianError::Io(e))?;
        }
        
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg("-R")
                .arg(&backup_info.backup_path)
                .spawn()
                .map_err(|e| SaveGuardianError::Io(e))?;
        }
        
        #[cfg(target_os = "linux")]
        {
            // Try to open the folder with the default file manager
            std::process::Command::new("xdg-open")
                .arg(folder_path)
                .spawn()
                .map_err(|e| SaveGuardianError::Io(e))?;
        }
        
        info!("Opened backup folder: {:?}", folder_path);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_count: usize,
    pub total_size: u64,
    pub steam_count: usize,
    pub non_steam_count: usize,
    pub oldest_backup: Option<chrono::DateTime<Utc>>,
    pub newest_backup: Option<chrono::DateTime<Utc>>,
}

impl BackupStats {
    pub fn format_total_size(&self) -> String {
        if self.total_size < 1024 {
            format!("{} B", self.total_size)
        } else if self.total_size < 1024 * 1024 {
            format!("{:.1} KB", self.total_size as f64 / 1024.0)
        } else if self.total_size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.total_size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", self.total_size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}