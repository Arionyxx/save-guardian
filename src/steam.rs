use crate::types::*;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
use log::{debug, info, warn};

pub struct SteamScanner {
    steam_userdata_path: PathBuf,
    app_cache: HashMap<u32, String>, // App ID -> Game Name
    cache_file_path: PathBuf,
}

impl SteamScanner {
    pub fn new(steam_path: PathBuf) -> Self {
        let cache_file_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("SaveGuardian")
            .join("steam_game_cache.json");
            
        let mut scanner = Self {
            steam_userdata_path: steam_path,
            app_cache: HashMap::new(),
            cache_file_path,
        };
        
        // Load existing cache from file
        scanner.load_cache();
        scanner
    }

    /// Scan for all Steam users and their saves
    pub fn scan_steam_saves(&mut self) -> Result<Vec<SteamUser>> {
        info!("Starting Steam save scan at {:?}", self.steam_userdata_path);
        
        if !self.steam_userdata_path.exists() {
            return Err(SaveGuardianError::PathNotFound(self.steam_userdata_path.clone()));
        }

        let mut users = Vec::new();
        
        // Read all directories in userdata (each is a Steam user)
        let entries = fs::read_dir(&self.steam_userdata_path)
            .map_err(|e| SaveGuardianError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| SaveGuardianError::Io(e))?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(user_id_str) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip non-numeric directories (like "anonymous")
                    if user_id_str.chars().all(|c| c.is_ascii_digit()) {
                        match self.scan_user_saves(user_id_str, &path) {
                            Ok(user) => {
                                info!("Found Steam user: {} with {} games", user_id_str, user.games.len());
                                users.push(user);
                            }
                            Err(e) => {
                                warn!("Failed to scan user {}: {}", user_id_str, e);
                            }
                        }
                    }
                }
            }
        }

        info!("Found {} Steam users total", users.len());
        Ok(users)
    }

    /// Scan saves for a specific Steam user
    fn scan_user_saves(&mut self, user_id: &str, user_path: &PathBuf) -> Result<SteamUser> {
        let mut games = Vec::new();
        
        // Read all app directories for this user
        let entries = fs::read_dir(user_path)
            .map_err(|e| SaveGuardianError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| SaveGuardianError::Io(e))?;
            let app_path = entry.path();
            
            if app_path.is_dir() {
                if let Some(app_id_str) = app_path.file_name().and_then(|n| n.to_str()) {
                    // Skip non-numeric directories
                    if let Ok(app_id) = app_id_str.parse::<u32>() {
                        if let Ok(mut app_games) = self.scan_app_saves(app_id, &app_path) {
                            games.append(&mut app_games);
                        }
                    }
                }
            }
        }

        Ok(SteamUser {
            id: user_id.to_string(),
            name: None, // We could potentially get this from Steam config files
            path: user_path.clone(),
            games,
        })
    }

    /// Scan saves for a specific Steam app
    fn scan_app_saves(&mut self, app_id: u32, app_path: &PathBuf) -> Result<Vec<GameSave>> {
        let mut saves = Vec::new();
        
        // Get proper game name from API/cache
        let game_name = self.get_game_name(app_id);
        
        // Only check the main remote folder to avoid duplicates
        // The "remote" folder is Steam's designated cloud save location
        let remote_path = app_path.join("remote");
        
        if remote_path.exists() && remote_path.is_dir() {
            // Use more lenient detection for the main save location
            if self.has_save_files_lenient(&remote_path)? {
                let save = GameSave::new(
                    game_name.clone(),
                    remote_path,
                    SaveType::Steam,
                    Some(app_id),
                );
                
                debug!("Found Steam save for app {}: {} at {:?}", app_id, save.name, save.save_path);
                saves.push(save);
            }
        }

        Ok(saves)
    }

    /// Check if a directory contains actual save files (not config/settings)
    fn has_save_files(&self, path: &PathBuf) -> Result<bool> {
        let walker = WalkDir::new(path)
            .max_depth(3) // Don't go too deep
            .follow_links(false);

        let mut found_actual_saves = false;
        let mut file_count = 0;

        for entry in walker {
            let entry = entry.map_err(|e| SaveGuardianError::Io(std::io::Error::from(e)))?;
            
            if entry.file_type().is_file() {
                file_count += 1;
                let file_path = entry.path();
                
                // Check for actual save file extensions (the main ones you want)
                if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = extension.to_lowercase();
                    if matches!(ext_lower.as_str(), 
                        "sav" | "save" | "savegame"
                    ) {
                        found_actual_saves = true;
                        break;
                    }
                }
                
                // Check for files that explicitly have "save" in the name (but not config/settings)
                if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                    let filename_lower = filename.to_lowercase();
                    if (filename_lower.contains("save") || filename_lower.contains("savegame")) &&
                       !filename_lower.contains("config") &&
                       !filename_lower.contains("settings") &&
                       !filename_lower.contains("cache") &&
                       !filename_lower.contains("temp") &&
                       !filename_lower.contains("log") {
                        found_actual_saves = true;
                        break;
                    }
                }
                
                // Stop checking after looking at too many files
                if file_count > 20 {
                    break;
                }
            }
        }

        Ok(found_actual_saves)
    }
    
    /// More lenient save file detection for main Steam remote folders
    fn has_save_files_lenient(&self, path: &PathBuf) -> Result<bool> {
        let walker = WalkDir::new(path)
            .max_depth(3) // Don't go too deep
            .follow_links(false);

        let mut file_count = 0;
        let mut has_files = false;

        for entry in walker {
            let entry = entry.map_err(|e| SaveGuardianError::Io(std::io::Error::from(e)))?;
            
            if entry.file_type().is_file() {
                file_count += 1;
                has_files = true;
                let file_path = entry.path();
                
                // Check for definitive save file extensions first
                if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = extension.to_lowercase();
                    if matches!(ext_lower.as_str(), 
                        "sav" | "save" | "savegame" | "dat" | "bin" | "json"
                    ) {
                        return Ok(true);
                    }
                }
                
                // Check for files that explicitly have "save" in the name
                if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                    let filename_lower = filename.to_lowercase();
                    if filename_lower.contains("save") || filename_lower.contains("savegame") {
                        return Ok(true);
                    }
                }
                
                // Stop checking after looking at too many files
                if file_count > 30 {
                    break;
                }
            }
        }

        // For remote folders, if we found any files at all, consider it a valid save location
        // This is because Steam's remote folder is the designated save sync location
        Ok(has_files && file_count > 0)
    }

    /// Get or generate a game name for the given app ID
    pub fn get_game_name(&mut self, app_id: u32) -> String {
        // Check if we have a cached name
        if let Some(cached_name) = self.app_cache.get(&app_id) {
            // Only use the cached name if it's not a generic fallback
            // Generic names usually start with "Unknown Game" or are clearly wrong
            if !cached_name.starts_with("Unknown Game") && 
               !cached_name.contains("(ac)") &&
               !self.is_likely_incorrect_name(cached_name, app_id) {
                return cached_name.clone();
            }
            // If the cached name looks wrong, we'll fetch a new one below
        }

        // Try to get the game name from Steam API or other sources
        let name = self.fetch_game_name_from_steam(app_id)
            .unwrap_or_else(|| format!("Unknown Game {}", app_id));
        
        // Cache the result and save to file
        self.app_cache.insert(app_id, name.clone());
        self.save_cache();
        
        name
    }
    
    /// Check if a cached name is likely incorrect and should be refetched
    fn is_likely_incorrect_name(&self, name: &str, app_id: u32) -> bool {
        // Check for generic patterns that indicate incorrect names
        name.starts_with("Unknown Game") ||
        name.contains("(ac)") ||
        name.contains("(workshop)") ||
        name.contains("(screenshots)") ||
        // Check if the name is just a number (app ID) which means it failed to get a real name
        name.parse::<u32>().is_ok() ||
        // Some other common incorrect patterns
        name.is_empty() ||
        name == "null" ||
        name.len() < 3 // Very short names are usually incorrect
    }
    
    /// Refresh incorrect names in the cache by re-fetching from API
    pub fn refresh_incorrect_names(&mut self) {
        let incorrect_entries: Vec<(u32, String)> = self.app_cache.iter()
            .filter(|(app_id, name)| self.is_likely_incorrect_name(name, **app_id))
            .map(|(app_id, name)| (*app_id, name.clone()))
            .collect();
        
        if !incorrect_entries.is_empty() {
            info!("Found {} incorrect cached names, refreshing...", incorrect_entries.len());
            
            for (app_id, old_name) in incorrect_entries {
                debug!("Refreshing incorrect name for {}: '{}'", app_id, old_name);
                if let Ok(new_name) = self.fetch_game_name_from_api(app_id) {
                    info!("Updated incorrect name for {}: '{}' -> '{}'", app_id, old_name, new_name);
                    self.app_cache.insert(app_id, new_name);
                } else {
                    // If API fails, at least remove the clearly wrong name
                    self.app_cache.remove(&app_id);
                }
                
                // Small delay to be respectful to APIs
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            
            self.save_cache();
        }
    }

    /// Attempt to fetch game name from Steam installation or online sources
    fn fetch_game_name_from_steam(&self, app_id: u32) -> Option<String> {
        // Try online APIs first (more reliable and up-to-date)
        debug!("Attempting to fetch game name for app ID {} from online sources", app_id);
        if let Ok(name) = self.fetch_game_name_from_api(app_id) {
            return Some(name);
        }
        
        // Try to read from Steam's registry (Windows)
        #[cfg(windows)]
        {
            if let Ok(game_name) = self.get_game_name_from_registry(app_id) {
                return Some(game_name);
            }
        }
        
        // Try to read from Steam's config files
        if let Ok(name) = self.get_game_name_from_config(app_id) {
            return Some(name);
        }
        
        None
    }
    
    /// Fetch game name from Steam API or SteamSpy API
    fn fetch_game_name_from_api(&self, app_id: u32) -> std::result::Result<String, Box<dyn std::error::Error>> {
        // Try Steam Store API first (free, no API key needed)
        if let Ok(name) = self.fetch_from_steam_store_api(app_id) {
            return Ok(name);
        }
        
        // Try SteamSpy API as fallback (also free)
        if let Ok(name) = self.fetch_from_steamspy_api(app_id) {
            return Ok(name);
        }
        
        Err("No API sources available".into())
    }
    
    /// Fetch game name from Steam Store API
    fn fetch_from_steam_store_api(&self, app_id: u32) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let url = format!("https://store.steampowered.com/api/appdetails?appids={}&filters=basic", app_id);
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        
        let response = client.get(&url)
            .header("User-Agent", "SaveGuardian/1.0")
            .send()?;
        
        if response.status().is_success() {
            let json: serde_json::Value = response.json()?;
            
            if let Some(app_data) = json.get(&app_id.to_string()) {
                if let Some(data) = app_data.get("data") {
                    if let Some(name) = data.get("name").and_then(|n| n.as_str()) {
                        info!("Fetched game name from Steam API: {} -> {}", app_id, name);
                        return Ok(name.to_string());
                    }
                }
            }
        }
        
        Err("Failed to get game name from Steam Store API".into())
    }
    
    /// Fetch game name from SteamSpy API as fallback
    fn fetch_from_steamspy_api(&self, app_id: u32) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let url = format!("https://steamspy.com/api.php?request=appdetails&appid={}", app_id);
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        
        let response = client.get(&url)
            .header("User-Agent", "SaveGuardian/1.0")
            .send()?;
        
        if response.status().is_success() {
            let json: serde_json::Value = response.json()?;
            
            if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
                if !name.is_empty() && name != "null" {
                    info!("Fetched game name from SteamSpy API: {} -> {}", app_id, name);
                    return Ok(name.to_string());
                }
            }
        }
        
        Err("Failed to get game name from SteamSpy API".into())
    }
    
    /// Load game name cache from file
    fn load_cache(&mut self) {
        if let Ok(cache_content) = fs::read_to_string(&self.cache_file_path) {
            if let Ok(cache) = serde_json::from_str::<HashMap<u32, String>>(&cache_content) {
                self.app_cache = cache;
                info!("Loaded {} game names from cache", self.app_cache.len());
            } else {
                warn!("Failed to parse game name cache file");
            }
        }
    }
    
    /// Save game name cache to file
    fn save_cache(&self) {
        // Ensure the directory exists
        if let Some(parent) = self.cache_file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        if let Ok(cache_json) = serde_json::to_string_pretty(&self.app_cache) {
            if let Err(e) = fs::write(&self.cache_file_path, cache_json) {
                warn!("Failed to save game name cache: {}", e);
            } else {
                debug!("Saved {} game names to cache", self.app_cache.len());
            }
        }
    }
    
    /// Refresh all cached game names by fetching them from online APIs
    pub fn refresh_game_names(&mut self) {
        info!("Refreshing {} cached game names...", self.app_cache.len());
        let app_ids: Vec<u32> = self.app_cache.keys().cloned().collect();
        
        let mut updated_count = 0;
        for app_id in app_ids {
            if let Ok(new_name) = self.fetch_game_name_from_api(app_id) {
                let old_name = self.app_cache.get(&app_id).cloned().unwrap_or_default();
                if old_name != new_name {
                    info!("Updated game name for {}: '{}' -> '{}'", app_id, old_name, new_name);
                    self.app_cache.insert(app_id, new_name);
                    updated_count += 1;
                }
            }
            
            // Small delay to be respectful to APIs
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        
        if updated_count > 0 {
            self.save_cache();
            info!("Updated {} game names in cache", updated_count);
        } else {
            info!("No game names needed updating");
        }
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, String) {
        let cache_size = self.app_cache.len();
        let cache_file_exists = self.cache_file_path.exists();
        let cache_path = self.cache_file_path.to_string_lossy().to_string();
        
        (cache_size, format!("Cache file: {} (exists: {})", cache_path, cache_file_exists))
    }
    
    /// Clear the game name cache (useful for troubleshooting)
    pub fn clear_cache(&mut self) {
        info!("Clearing game name cache ({} entries)", self.app_cache.len());
        self.app_cache.clear();
        
        // Remove the cache file
        if self.cache_file_path.exists() {
            if let Err(e) = fs::remove_file(&self.cache_file_path) {
                warn!("Failed to remove cache file: {}", e);
            } else {
                info!("Cache file removed successfully");
            }
        }
    }

    #[cfg(windows)]
    fn get_game_name_from_registry(&self, app_id: u32) -> std::result::Result<String, Box<dyn std::error::Error>> {
        use winreg::{RegKey, enums::*};
        
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let steam_apps = hklm.open_subkey(r"SOFTWARE\Valve\Steam\Apps")?;
        let app_key = steam_apps.open_subkey(app_id.to_string())?;
        let name: String = app_key.get_value("Name")?;
        Ok(name)
    }

    fn get_game_name_from_config(&self, _app_id: u32) -> std::result::Result<String, Box<dyn std::error::Error>> {
        // This could be implemented to read from Steam's localconfig.vdf or other files
        // For now, we'll just return an error to fall back to the default naming
        Err("Not implemented".into())
    }

    /// Load known game names from a comprehensive database
    pub fn load_game_database(&mut self) {
        let common_games = vec![
            // Popular multiplayer games
            (570, "Dota 2"),
            (730, "Counter-Strike: Global Offensive"),
            (440, "Team Fortress 2"),
            (578080, "PLAYERUNKNOWN'S BATTLEGROUNDS"),
            (252490, "Rust"),
            (377160, "Fall Guys"),
            (1172470, "Apex Legends"),
            (1938090, "Call of Duty: Warzone 2.0"),
            
            // Open world/RPG games
            (271590, "Grand Theft Auto V"),
            (292030, "The Witcher 3: Wild Hunt"),
            (367520, "Hollow Knight"),
            (431960, "Wallpaper Engine"),
            (1174180, "Red Dead Redemption 2"),
            (435150, "Divinity: Original Sin 2"),
            (489830, "The Elder Scrolls V: Skyrim Special Edition"),
            (377160, "Fallout 4"),
            
            // Survival/Crafting games
            (896660, "Valheim"),
            (892970, "Valheim Dedicated Server"),
            (1063730, "New World"),
            (548430, "Deep Rock Galactic"),
            (674940, "Stick Fight: The Game"),
            
            // Horror games
            (1325200, "Phasmophobia"),
            (739630, "Phasmophobia Beta"),
            (291550, "Brawlhalla"),
            
            // Indie/Popular games
            (413150, "Stardew Valley"),
            (646570, "Slay the Spire"),
            (1091500, "Cyberpunk 2077"),
            (620980, "Beat Saber"),
            (322330, "Don't Starve Together"),
            
            // Dying Light series
            (239140, "Dying Light"),
            (881020, "Dying Light: The Following"),
            (534380, "Dying Light: Bad Blood"),
            (1966720, "Dying Light 2 Stay Human"),
            
            // Strategy games
            (394360, "Hearts of Iron IV"),
            (281990, "Stellaris"),
            (236850, "Europa Universalis IV"),
            (1158310, "Crusader Kings III"),
            
            // Simulation games
            (255710, "Cities: Skylines"),
            (823500, "Satisfactory"),
            (544550, "Kaspersky Rescue Disk"),
            
            // Fighting games
            (1384160, "Street Fighter 6"),
            (1778820, "Tekken 8"),
            (582010, "Monster Hunter: World"),
            
            // Racing games
            (1551360, "Forza Horizon 5"),
            (1293830, "Forza Horizon 4"),
            
            // Minecraft-related (Java)
            (1172620, "Sea of Thieves"),
            (1174180, "Red Dead Redemption 2"),
            
            // VR Games
            (546560, "Half-Life: Alyx"),
            (620980, "Beat Saber"),
            
            // Popular Steam games
            (359550, "Tom Clancy's Rainbow Six Siege"),
            (550, "Left 4 Dead 2"),
            (4000, "Garry's Mod"),
            (105600, "Terraria"),
            (72850, "The Elder Scrolls V: Skyrim"),
            (8930, "Sid Meier's Civilization V"),
            (289070, "Sid Meier's Civilization VI"),
            (812140, "Assassin's Creed Odyssey"),
            (881100, "Assassin's Creed Origins"),
            (1693980, "Assassin's Creed Mirage"),
            
            // MMOs
            (306130, "The Elder Scrolls Online"),
            (39120, "Final Fantasy XIV Online"),
            
            // Popular indie games
            (230410, "Warframe"),
            (238960, "Path of Exile"),
            (431960, "Wallpaper Engine"),
            (381210, "Dead by Daylight"),
            
            // Newer releases
            (1938090, "Call of Duty: Modern Warfare II"),
            (1449850, "Yu-Gi-Oh! Master Duel"),
            (1172470, "Apex Legends"),
            (1091500, "Cyberpunk 2077"),
        ];

        for (app_id, name) in common_games {
            self.app_cache.insert(app_id, name.to_string());
        }
        
        info!("Loaded {} game names into cache", self.app_cache.len());
    }

    /// Get Steam installation path from registry
    #[cfg(windows)]
    pub fn get_steam_install_path() -> Option<PathBuf> {
        use winreg::{RegKey, enums::*};
        
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(steam_key) = hklm.open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam") {
            if let Ok(install_path) = steam_key.get_value::<String, _>("InstallPath") {
                return Some(PathBuf::from(install_path).join("userdata"));
            }
        }
        
        // Fallback to common location
        Some(PathBuf::from(r"C:\Program Files (x86)\Steam\userdata"))
    }

    #[cfg(not(windows))]
    pub fn get_steam_install_path() -> Option<PathBuf> {
        // Linux/Mac Steam paths
        if let Some(home) = dirs::home_dir() {
            let linux_path = home.join(".local/share/Steam/userdata");
            if linux_path.exists() {
                return Some(linux_path);
            }
            
            let mac_path = home.join("Library/Application Support/Steam/userdata");
            if mac_path.exists() {
                return Some(mac_path);
            }
        }
        None
    }
}