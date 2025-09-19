use crate::types::*;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
use chrono::Utc;
use log::{debug, info, warn};

pub struct SyncManager {
    backup_before_sync: bool,
}

impl SyncManager {
    pub fn new(backup_before_sync: bool) -> Self {
        Self {
            backup_before_sync,
        }
    }

    /// Find potential sync pairs between Steam and non-Steam saves
    pub fn find_sync_pairs(&self, steam_saves: &[GameSave], non_steam_saves: &[GameSave]) -> Vec<SyncPair> {
        let mut sync_pairs = Vec::new();

        // First, try to match by app ID (for games that might have both Steam and non-Steam versions)
        for steam_save in steam_saves {
            if let Some(app_id) = steam_save.app_id {
                // Look for non-Steam saves with similar names that might match this Steam game
                for non_steam_save in non_steam_saves {
                    if self.is_likely_same_game(&steam_save.name, &non_steam_save.name, Some(app_id)) {
                        sync_pairs.push(SyncPair {
                            steam_save: Some(steam_save.clone()),
                            non_steam_save: Some(non_steam_save.clone()),
                            game_name: steam_save.name.clone(),
                            app_id: Some(app_id),
                            last_synced: None,
                            sync_direction: SyncDirection::Bidirectional,
                        });
                    }
                }
            }
        }

        // Then, try to match by game name similarity for games without clear app ID matches
        for steam_save in steam_saves {
            let already_paired = sync_pairs.iter().any(|pair| {
                pair.steam_save.as_ref().map(|s| &s.save_path) == Some(&steam_save.save_path)
            });

            if !already_paired {
                for non_steam_save in non_steam_saves {
                    let already_paired_ns = sync_pairs.iter().any(|pair| {
                        pair.non_steam_save.as_ref().map(|s| &s.save_path) == Some(&non_steam_save.save_path)
                    });

                    if !already_paired_ns && self.is_likely_same_game(&steam_save.name, &non_steam_save.name, steam_save.app_id) {
                        sync_pairs.push(SyncPair {
                            steam_save: Some(steam_save.clone()),
                            non_steam_save: Some(non_steam_save.clone()),
                            game_name: self.get_common_game_name(&steam_save.name, &non_steam_save.name),
                            app_id: steam_save.app_id,
                            last_synced: None,
                            sync_direction: SyncDirection::Bidirectional,
                        });
                        break;
                    }
                }
            }
        }

        // Add unpaired Steam saves
        for steam_save in steam_saves {
            let already_paired = sync_pairs.iter().any(|pair| {
                pair.steam_save.as_ref().map(|s| &s.save_path) == Some(&steam_save.save_path)
            });

            if !already_paired {
                sync_pairs.push(SyncPair {
                    steam_save: Some(steam_save.clone()),
                    non_steam_save: None,
                    game_name: steam_save.name.clone(),
                    app_id: steam_save.app_id,
                    last_synced: None,
                    sync_direction: SyncDirection::SteamToNonSteam,
                });
            }
        }

        // Add unpaired non-Steam saves
        for non_steam_save in non_steam_saves {
            let already_paired = sync_pairs.iter().any(|pair| {
                pair.non_steam_save.as_ref().map(|s| &s.save_path) == Some(&non_steam_save.save_path)
            });

            if !already_paired {
                sync_pairs.push(SyncPair {
                    steam_save: None,
                    non_steam_save: Some(non_steam_save.clone()),
                    game_name: non_steam_save.name.clone(),
                    app_id: None,
                    last_synced: None,
                    sync_direction: SyncDirection::NonSteamToSteam,
                });
            }
        }

        info!("Found {} potential sync pairs", sync_pairs.len());
        sync_pairs
    }

    /// Synchronize saves between Steam and non-Steam versions
    pub fn sync_saves(
        &self,
        sync_pair: &mut SyncPair,
        direction: SyncDirection,
        backup_manager: Option<&crate::backup::BackupManager>,
    ) -> Result<SyncResult> {
        info!("Syncing saves for {} in direction {:?}", sync_pair.game_name, direction);

        let (source, destination) = match direction {
            SyncDirection::SteamToNonSteam => {
                match (&sync_pair.steam_save, &sync_pair.non_steam_save) {
                    (Some(steam), Some(non_steam)) => (steam, non_steam),
                    (Some(steam), None) => {
                        return Err(SaveGuardianError::SaveOperationFailed(
                            "No non-Steam save location specified".to_string()
                        ));
                    }
                    _ => {
                        return Err(SaveGuardianError::SaveOperationFailed(
                            "No Steam save found to sync from".to_string()
                        ));
                    }
                }
            }
            SyncDirection::NonSteamToSteam => {
                match (&sync_pair.non_steam_save, &sync_pair.steam_save) {
                    (Some(non_steam), Some(steam)) => (non_steam, steam),
                    (Some(non_steam), None) => {
                        return Err(SaveGuardianError::SaveOperationFailed(
                            "No Steam save location specified".to_string()
                        ));
                    }
                    _ => {
                        return Err(SaveGuardianError::SaveOperationFailed(
                            "No non-Steam save found to sync from".to_string()
                        ));
                    }
                }
            }
            SyncDirection::Bidirectional => {
                // For bidirectional sync, determine direction based on modification time
                match (&sync_pair.steam_save, &sync_pair.non_steam_save) {
                    (Some(steam), Some(non_steam)) => {
                        let steam_time = steam.last_modified.unwrap_or(chrono::DateTime::from_timestamp(0, 0).unwrap());
                        let non_steam_time = non_steam.last_modified.unwrap_or(chrono::DateTime::from_timestamp(0, 0).unwrap());
                        
                        if steam_time > non_steam_time {
                            (steam, non_steam)
                        } else {
                            (non_steam, steam)
                        }
                    }
                    _ => {
                        return Err(SaveGuardianError::SaveOperationFailed(
                            "Both save locations required for bidirectional sync".to_string()
                        ));
                    }
                }
            }
        };

        // Create backup if requested and backup manager is available
        if self.backup_before_sync {
            if let Some(bm) = backup_manager {
                match bm.create_backup(destination, Some("Pre-sync backup".to_string())) {
                    Ok(_) => info!("Created pre-sync backup for {}", destination.name),
                    Err(e) => warn!("Failed to create pre-sync backup: {}", e),
                }
            }
        }

        // Perform the actual sync operation
        let files_copied = self.copy_save_files(&source.save_path, &destination.save_path)?;

        // Update sync information
        sync_pair.last_synced = Some(Utc::now());
        sync_pair.sync_direction = direction;

        Ok(SyncResult {
            files_copied,
            bytes_copied: self.calculate_directory_size(&destination.save_path)?,
            source_path: source.save_path.clone(),
            destination_path: destination.save_path.clone(),
            sync_time: Utc::now(),
        })
    }

    /// Copy save files from source to destination
    fn copy_save_files(&self, source: &PathBuf, destination: &PathBuf) -> Result<usize> {
        info!("Copying save files from {:?} to {:?}", source, destination);

        // Create destination directory if it doesn't exist
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SaveGuardianError::SaveOperationFailed(format!("Failed to create destination directory: {}", e)))?;
        }

        let mut files_copied = 0;

        if source.is_file() {
            // Copy single file
            if let Some(filename) = source.file_name() {
                let dest_file = destination.join(filename);
                fs::copy(source, &dest_file)
                    .map_err(|e| SaveGuardianError::SaveOperationFailed(format!("Failed to copy file: {}", e)))?;
                files_copied = 1;
                debug!("Copied file: {:?} -> {:?}", source, dest_file);
            }
        } else if source.is_dir() {
            // Copy directory recursively
            
            // First, remove existing files in destination if it exists
            if destination.exists() {
                fs::remove_dir_all(destination)
                    .map_err(|e| SaveGuardianError::SaveOperationFailed(format!("Failed to remove existing destination: {}", e)))?;
            }

            // Create destination directory
            fs::create_dir_all(destination)
                .map_err(|e| SaveGuardianError::SaveOperationFailed(format!("Failed to create destination directory: {}", e)))?;

            let walker = WalkDir::new(source)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok());

            for entry in walker {
                let path = entry.path();
                let relative_path = path.strip_prefix(source)
                    .map_err(|e| SaveGuardianError::SaveOperationFailed(format!("Path error: {}", e)))?;

                let dest_path = destination.join(&relative_path);

                if path.is_file() {
                    // Create parent directories if needed
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| SaveGuardianError::SaveOperationFailed(format!("Failed to create parent directory: {}", e)))?;
                    }

                    // Copy the file
                    fs::copy(path, &dest_path)
                        .map_err(|e| SaveGuardianError::SaveOperationFailed(format!("Failed to copy file: {}", e)))?;
                    
                    files_copied += 1;
                    debug!("Copied file: {:?} -> {:?}", path, dest_path);
                } else if path.is_dir() && relative_path.as_os_str() != "" {
                    // Create directory
                    fs::create_dir_all(&dest_path)
                        .map_err(|e| SaveGuardianError::SaveOperationFailed(format!("Failed to create directory: {}", e)))?;
                    
                    debug!("Created directory: {:?}", dest_path);
                }
            }
        } else {
            return Err(SaveGuardianError::SaveOperationFailed(
                "Source path is neither file nor directory".to_string()
            ));
        }

        info!("Successfully copied {} files", files_copied);
        Ok(files_copied)
    }

    /// Calculate the total size of a directory
    fn calculate_directory_size(&self, path: &PathBuf) -> Result<u64> {
        let mut total_size = 0;

        if path.is_file() {
            total_size = path.metadata()
                .map_err(|e| SaveGuardianError::Io(e))?
                .len();
        } else if path.is_dir() {
            let walker = WalkDir::new(path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok());

            for entry in walker {
                if entry.file_type().is_file() {
                    total_size += entry.metadata()
                        .map_err(|e| SaveGuardianError::Io(std::io::Error::from(e)))?
                        .len();
                }
            }
        }

        Ok(total_size)
    }

    /// Check if two game names likely refer to the same game
    fn is_likely_same_game(&self, name1: &str, name2: &str, app_id: Option<u32>) -> bool {
        // Normalize names for comparison
        let norm1 = self.normalize_game_name(name1);
        let norm2 = self.normalize_game_name(name2);

        // Exact match
        if norm1 == norm2 {
            return true;
        }

        // Check if one name contains the other
        if norm1.contains(&norm2) || norm2.contains(&norm1) {
            return true;
        }

        // Check for common game name variations
        if self.check_common_variations(&norm1, &norm2) {
            return true;
        }

        // If we have an app ID, check against known game mappings
        if let Some(id) = app_id {
            if self.check_app_id_name_match(id, &norm2) {
                return true;
            }
        }

        // Calculate similarity score
        let similarity = self.calculate_string_similarity(&norm1, &norm2);
        similarity > 0.7 // 70% similarity threshold
    }

    /// Normalize game name for comparison
    fn normalize_game_name(&self, name: &str) -> String {
        name.to_lowercase()
            .replace(['-', '_', ':', '!', '?'], " ")
            .split_whitespace()
            .filter(|word| !matches!(*word, "the" | "a" | "an" | "and" | "or" | "of" | "in" | "on" | "at" | "to" | "for" | "with"))
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    }

    /// Check for common game name variations
    fn check_common_variations(&self, name1: &str, name2: &str) -> bool {
        let variations = vec![
            ("goty", "game of the year"),
            ("deluxe", "deluxe edition"),
            ("ultimate", "ultimate edition"),
            ("remastered", "remaster"),
            ("enhanced", "enhanced edition"),
            ("definitive", "definitive edition"),
            ("directors", "director's cut"),
            ("complete", "complete edition"),
        ];

        for (short, long) in variations {
            if (name1.contains(short) && name2.contains(long)) || 
               (name1.contains(long) && name2.contains(short)) {
                return true;
            }
        }

        false
    }

    /// Check if an app ID matches a game name
    fn check_app_id_name_match(&self, app_id: u32, name: &str) -> bool {
        // This could be expanded with a comprehensive database of Steam app IDs to game names
        let known_mappings = vec![
            (239140, "dying light"),
            (881020, "dying light"),
            (271590, "grand theft auto"),
            (271590, "gta"),
            (730, "counter strike"),
            (730, "cs go"),
            (440, "team fortress"),
            (570, "dota"),
        ];

        for (id, game_name) in known_mappings {
            if id == app_id && name.contains(game_name) {
                return true;
            }
        }

        false
    }

    /// Calculate string similarity using a simple algorithm
    fn calculate_string_similarity(&self, s1: &str, s2: &str) -> f64 {
        if s1.is_empty() && s2.is_empty() {
            return 1.0;
        }
        
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }

        let len1 = s1.len();
        let len2 = s2.len();
        let max_len = len1.max(len2);

        // Simple Levenshtein distance calculation
        let distance = self.levenshtein_distance(s1, s2);
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();

        if len1 == 0 { return len2; }
        if len2 == 0 { return len1; }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i-1] == chars2[j-1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i-1][j] + 1)
                    .min(matrix[i][j-1] + 1)
                    .min(matrix[i-1][j-1] + cost);
            }
        }

        matrix[len1][len2]
    }

    /// Get a common game name from two similar names
    fn get_common_game_name(&self, name1: &str, name2: &str) -> String {
        // Use the shorter, cleaner name
        if name1.len() <= name2.len() {
            name1.to_string()
        } else {
            name2.to_string()
        }
    }

    /// Create a sync pair manually
    pub fn create_manual_sync_pair(
        &self,
        steam_save: Option<GameSave>,
        non_steam_save: Option<GameSave>,
        custom_name: Option<String>,
    ) -> Result<SyncPair> {
        let game_name = match (&steam_save, &non_steam_save, custom_name) {
            (_, _, Some(name)) => name,
            (Some(steam), _, None) => steam.name.clone(),
            (None, Some(non_steam), None) => non_steam.name.clone(),
            (None, None, None) => return Err(SaveGuardianError::SaveOperationFailed(
                "At least one save location must be provided".to_string()
            )),
        };

        let app_id = steam_save.as_ref().and_then(|s| s.app_id);

        let sync_direction = match (&steam_save, &non_steam_save) {
            (Some(_), Some(_)) => SyncDirection::Bidirectional,
            (Some(_), None) => SyncDirection::SteamToNonSteam,
            (None, Some(_)) => SyncDirection::NonSteamToSteam,
            (None, None) => return Err(SaveGuardianError::SaveOperationFailed(
                "At least one save location must be provided".to_string()
            )),
        };

        Ok(SyncPair {
            steam_save,
            non_steam_save,
            game_name,
            app_id,
            last_synced: None,
            sync_direction,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub files_copied: usize,
    pub bytes_copied: u64,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub sync_time: chrono::DateTime<Utc>,
}

impl SyncResult {
    pub fn format_bytes_copied(&self) -> String {
        if self.bytes_copied < 1024 {
            format!("{} B", self.bytes_copied)
        } else if self.bytes_copied < 1024 * 1024 {
            format!("{:.1} KB", self.bytes_copied as f64 / 1024.0)
        } else if self.bytes_copied < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.bytes_copied as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", self.bytes_copied as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}