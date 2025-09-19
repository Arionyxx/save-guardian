use crate::types::*;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
use log::{debug, info, warn};

pub struct NonSteamScanner {
    common_locations: Vec<SaveLocation>,
    custom_locations: Vec<SaveLocation>,
}

impl NonSteamScanner {
    pub fn new() -> Self {
        Self {
            common_locations: Self::get_default_locations(),
            custom_locations: Vec::new(),
        }
    }

    pub fn with_custom_locations(mut self, custom_locations: Vec<SaveLocation>) -> Self {
        self.custom_locations = custom_locations;
        self
    }

    /// Get default common save locations for Windows
    fn get_default_locations() -> Vec<SaveLocation> {
        let mut locations = Vec::new();
        
        if let Some(home) = dirs::home_dir() {
            // Documents locations
            let documents = dirs::document_dir().unwrap_or_else(|| home.join("Documents"));
            
            locations.extend(vec![
                SaveLocation {
                    path: documents.join("My Games"),
                    location_type: LocationType::Documents,
                    description: "Documents\\My Games - Common for many PC games".to_string(),
                    is_custom: false,
                },
                SaveLocation {
                    path: documents.clone(),
                    location_type: LocationType::Documents,
                    description: "Documents - Direct saves in Documents folder".to_string(),
                    is_custom: false,
                },
                SaveLocation {
                    path: documents.join("Rockstar Games"),
                    location_type: LocationType::Documents,
                    description: "Documents\\Rockstar Games - Rockstar titles".to_string(),
                    is_custom: false,
                },
            ]);

            // AppData Roaming
            if let Some(roaming) = dirs::config_dir() {
                locations.push(SaveLocation {
                    path: roaming,
                    location_type: LocationType::AppDataRoaming,
                    description: "AppData\\Roaming - Config and saves for many games".to_string(),
                    is_custom: false,
                });
            }

            // AppData Local
            if let Some(local) = dirs::cache_dir() {
                locations.push(SaveLocation {
                    path: local,
                    location_type: LocationType::AppDataLocal,
                    description: "AppData\\Local - Modern game saves and settings".to_string(),
                    is_custom: false,
                });
            }

            // AppData LocalLow (Unity games)
            let locallow = home.join("AppData").join("LocalLow");
            if locallow.exists() {
                locations.push(SaveLocation {
                    path: locallow,
                    location_type: LocationType::AppDataLocalLow,
                    description: "AppData\\LocalLow - Unity games persistent data".to_string(),
                    is_custom: false,
                });
            }

            // Public Documents
            let public_docs = PathBuf::from(r"C:\Users\Public\Documents");
            if public_docs.exists() {
                locations.push(SaveLocation {
                    path: public_docs,
                    location_type: LocationType::PublicDocuments,
                    description: "Public Documents - Some cracks and older titles".to_string(),
                    is_custom: false,
                });
            }

            // Goldberg Steam Emu saves
            if let Some(roaming) = dirs::config_dir() {
                let goldberg_path = roaming.join("Goldberg SteamEmu Saves");
                locations.push(SaveLocation {
                    path: goldberg_path,
                    location_type: LocationType::AppDataRoaming,
                    description: "Goldberg SteamEmu Saves - Emulated Steam saves".to_string(),
                    is_custom: false,
                });
            }
        }

        locations
    }

    /// Scan for non-Steam game saves
    pub fn scan_non_steam_saves(&self) -> Result<Vec<GameSave>> {
        info!("Starting non-Steam save scan");
        let mut all_saves = Vec::new();

        // Scan common locations
        for location in &self.common_locations {
            if let Ok(mut saves) = self.scan_location(location) {
                info!("Found {} saves in {}", saves.len(), location.description);
                all_saves.append(&mut saves);
            }
        }

        // Scan custom locations
        for location in &self.custom_locations {
            if let Ok(mut saves) = self.scan_location(location) {
                info!("Found {} saves in custom location: {}", saves.len(), location.description);
                all_saves.append(&mut saves);
            }
        }

        info!("Found {} total non-Steam saves", all_saves.len());
        Ok(all_saves)
    }

    /// Scan a specific location for game saves
    fn scan_location(&self, location: &SaveLocation) -> Result<Vec<GameSave>> {
        if !location.path.exists() {
            debug!("Location does not exist: {:?}", location.path);
            return Ok(Vec::new());
        }

        let mut saves = Vec::new();
        let walker = WalkDir::new(&location.path)
            .max_depth(4) // Don't go too deep to avoid performance issues
            .follow_links(false);

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Error walking directory: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            
            // Skip if it's not a directory
            if !path.is_dir() {
                continue;
            }

            // Check if this directory looks like it contains game saves
            if self.is_potential_game_save_directory(path)? {
                if let Some(game_name) = self.extract_game_name_from_path(path) {
                    let save = GameSave::new(
                        game_name,
                        path.to_path_buf(),
                        SaveType::NonSteam,
                        None, // Non-Steam games don't have app IDs
                    );
                    
                    debug!("Found non-Steam save: {} at {:?}", save.name, save.save_path);
                    saves.push(save);
                }
            }
        }

        Ok(saves)
    }

    /// Check if a directory contains actual game save files
    fn is_potential_game_save_directory(&self, path: &std::path::Path) -> Result<bool> {
        // Check for actual save files
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return Ok(false),
        };

        let mut has_actual_saves = false;
        let mut file_count = 0;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let file_path = entry.path();
            file_count += 1;

            if file_path.is_file() {
                // Check for actual save file extensions first
                if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = extension.to_lowercase();
                    if matches!(ext_lower.as_str(),
                        "sav" | "save" | "savegame"
                    ) {
                        has_actual_saves = true;
                        break;
                    }
                }
                
                // Check for files with "save" in name but exclude config/settings files
                if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                    let filename_lower = filename.to_lowercase();
                    
                    if (filename_lower.contains("save") || filename_lower.contains("savegame")) &&
                       !filename_lower.contains("config") &&
                       !filename_lower.contains("settings") &&
                       !filename_lower.contains("cache") &&
                       !filename_lower.contains("temp") &&
                       !filename_lower.contains("log") &&
                       !filename_lower.contains("backup") &&
                       !filename_lower.ends_with(".jar") &&
                       !filename_lower.ends_with(".java") &&
                       !filename_lower.contains("version") {
                        has_actual_saves = true;
                        break;
                    }
                }
            }

            // Don't check too many files to avoid performance issues
            if file_count > 30 {
                break;
            }
        }

        // Must have actual save files and not be a system directory
        Ok(has_actual_saves && !self.is_system_directory(path))
    }

    /// Check if a directory is a system directory that should be ignored
    fn is_system_directory(&self, path: &std::path::Path) -> bool {
        if let Some(path_str) = path.to_str() {
            let path_lower = path_str.to_lowercase();
            
            // Skip system directories and development-related paths
            if path_lower.contains("windows") ||
               path_lower.contains("system32") ||
               path_lower.contains("program files") ||
               path_lower.contains("programdata") ||
               path_lower.contains("microsoft") ||
               path_lower.contains("adobe") ||
               path_lower.contains("google") ||
               path_lower.contains("mozilla") ||
               path_lower.contains("temp") ||
               path_lower.contains("cache") ||
               path_lower.contains("logs") ||
               path_lower.contains("crash") ||
               // Minecraft-specific exclusions
               path_lower.contains("minecraft") ||
               path_lower.contains(".minecraft") ||
               path_lower.contains("mods") ||
               path_lower.contains("versions") ||
               path_lower.contains("libraries") ||
               // Development/IDE exclusions
               path_lower.contains("node_modules") ||
               path_lower.contains(".git") ||
               path_lower.contains("target") ||
               path_lower.contains("build") ||
               path_lower.contains("bin") ||
               path_lower.contains("obj") ||
               path_lower.contains(".vs") ||
               path_lower.contains("__pycache__") {
                return true;
            }
        }

        false
    }

    /// Extract game name from the directory path
    fn extract_game_name_from_path(&self, path: &std::path::Path) -> Option<String> {
        // Try to get the most specific directory name that represents the game
        let components: Vec<_> = path.components().collect();
        
        // Look for game-specific patterns in the path
        for component in components.iter().rev() {
            if let Some(name) = component.as_os_str().to_str() {
                let name_lower = name.to_lowercase();
                
                // Skip common non-game directory names
                if matches!(name_lower.as_str(),
                    "saves" | "save" | "profiles" | "profile" | "data" | "config" |
                    "settings" | "user" | "users" | "documents" | "my games" |
                    "appdata" | "roaming" | "local" | "locallow" | "public" |
                    "remote" | "steam" | "steamemu" | "goldberg" | "minecraft" |
                    "versions" | "mods" | "libraries" | "bin" | "temp" | "cache"
                ) {
                    continue;
                }
                
                // Skip version-like names (numbers with dots) and Minecraft versions
                if name_lower.matches('.').count() >= 2 || // like "1.20.1"
                   name_lower.starts_with("1.") || // Minecraft versions
                   name_lower.contains("-forge") ||
                   name_lower.contains("-fabric") ||
                   name_lower.contains("optifine") ||
                   name_lower.contains("pre") && name_lower.len() < 10 { // like "pre3"
                    continue;
                }

                // This looks like a potential game name
                return Some(self.clean_game_name(name));
            }
        }

        // Fallback: use the last directory name
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| self.clean_game_name(s))
    }

    /// Clean up the game name by removing common suffixes and formatting
    fn clean_game_name(&self, name: &str) -> String {
        let mut clean_name = name.to_string();

        // Remove common version suffixes
        let suffixes_to_remove = vec![
            " - Save", " - Saves", " Save", " Saves",
            " - Config", " Config", " - Settings", " Settings",
            " - Profile", " Profile", " Profiles",
            " (Steam)", " (Non-Steam)", " (Cracked)",
        ];

        for suffix in suffixes_to_remove {
            if clean_name.ends_with(suffix) {
                clean_name = clean_name[..clean_name.len() - suffix.len()].to_string();
            }
        }

        // Replace underscores with spaces and title case
        clean_name = clean_name.replace('_', " ");
        
        // Simple title case
        clean_name = clean_name
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        clean_name.trim().to_string()
    }

    /// Add a custom save location
    pub fn add_custom_location(&mut self, location: SaveLocation) {
        self.custom_locations.push(location);
    }

    /// Remove a custom save location
    pub fn remove_custom_location(&mut self, path: &PathBuf) {
        self.custom_locations.retain(|loc| &loc.path != path);
    }

    /// Get all configured locations
    pub fn get_all_locations(&self) -> Vec<&SaveLocation> {
        let mut all_locations = Vec::new();
        all_locations.extend(&self.common_locations);
        all_locations.extend(&self.custom_locations);
        all_locations
    }

    /// Scan a specific game directory (useful for game install directories)
    pub fn scan_game_install_directory(&self, game_path: &PathBuf, game_name: &str) -> Result<Option<GameSave>> {
        if !game_path.exists() {
            return Ok(None);
        }

        // Common save subdirectories in game installations
        let save_subdirs = vec!["Save", "Saves", "Saved", "Profile", "Profiles", "Data", "User"];
        
        for subdir in save_subdirs {
            let save_path = game_path.join(subdir);
            if save_path.exists() && save_path.is_dir() {
                if self.is_potential_game_save_directory(&save_path)? {
                    return Ok(Some(GameSave::new(
                        format!("{} (Install)", game_name),
                        save_path,
                        SaveType::NonSteam,
                        None,
                    )));
                }
            }
        }

        Ok(None)
    }
}