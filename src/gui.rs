use crate::types::*;
use crate::steam::SteamScanner;
use crate::non_steam::NonSteamScanner;
use crate::backup::{BackupManager, BackupStats};
use crate::sync::{SyncManager, SyncResult};
use eframe::egui;
use log::{error, info, warn};
use chrono::Utc;

pub struct SaveGuardianApp {
    // Core managers
    steam_scanner: SteamScanner,
    non_steam_scanner: NonSteamScanner,
    backup_manager: Option<BackupManager>,
    sync_manager: SyncManager,
    
    // Application state
    config: Config,
    steam_saves: Vec<GameSave>,
    non_steam_saves: Vec<GameSave>,
    sync_pairs: Vec<SyncPair>,
    backups: Vec<BackupInfo>,
    backup_stats: Option<BackupStats>,
    
    // UI state
    selected_tab: Tab,
    selected_game: Option<usize>,
    selected_backup: Option<usize>,
    selected_sync_pair: Option<usize>,
    scan_status: ScanStatus,
    last_sync_result: Option<SyncResult>,
    
    // Dialogs and modals
    show_settings: bool,
    show_backup_dialog: bool,
    show_restore_dialog: bool,
    show_sync_dialog: bool,
    show_about: bool,
    
    // Settings UI
    temp_config: Config,
    
    // Search and filters
    search_query: String,
    filter_steam: bool,
    filter_non_steam: bool,
    sort_by: SortBy,
    
    // Cloud sync tracking
    last_sync_time: Option<chrono::DateTime<chrono::Utc>>,
    cloud_files_synced: usize,
    cloud_storage_used: u64,
}

#[derive(Debug, Clone, PartialEq)]
enum Tab {
    GameSaves,
    Backups,
    Sync,
    Cloud,
    Settings,
}

#[derive(Debug, Clone)]
enum ScanStatus {
    Idle,
    Scanning,
    Complete(String),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
enum SortBy {
    Name,
    LastModified,
    Size,
    Type,
}

impl Default for SaveGuardianApp {
    fn default() -> Self {
        let config = Config::default();
        let steam_scanner = SteamScanner::new(config.steam_path.clone());
        let non_steam_scanner = NonSteamScanner::new();
        let backup_manager = BackupManager::new(config.backup_path.clone(), config.backup_retention_days).ok();
        let sync_manager = SyncManager::new(true); // Enable backup before sync by default

        Self {
            steam_scanner,
            non_steam_scanner,
            backup_manager,
            sync_manager,
            config: config.clone(),
            steam_saves: Vec::new(),
            non_steam_saves: Vec::new(),
            sync_pairs: Vec::new(),
            backups: Vec::new(),
            backup_stats: None,
            selected_tab: Tab::GameSaves,
            selected_game: None,
            selected_backup: None,
            selected_sync_pair: None,
            scan_status: ScanStatus::Idle,
            last_sync_result: None,
            show_settings: false,
            show_backup_dialog: false,
            show_restore_dialog: false,
            show_sync_dialog: false,
            show_about: false,
            temp_config: config,
            search_query: String::new(),
            filter_steam: true,
            filter_non_steam: true,
            sort_by: SortBy::Name,
            last_sync_time: None,
            cloud_files_synced: 0,
            cloud_storage_used: 0,
        }
    }
}

impl eframe::App for SaveGuardianApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme
        self.apply_theme(ctx);
        
        // Top panel with title and controls
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.draw_top_panel(ui);
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.draw_status_bar(ui);
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::GameSaves => self.draw_game_saves_tab(ui),
                Tab::Backups => self.draw_backups_tab(ui),
                Tab::Sync => self.draw_sync_tab(ui),
                Tab::Cloud => self.draw_cloud_tab(ui),
                Tab::Settings => self.draw_settings_tab(ui),
            }
        });

        // Modal dialogs
        self.draw_modals(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "save_guardian_config", &self.config);
    }
}

impl SaveGuardianApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        
        // Load saved configuration
        if let Some(storage) = cc.storage {
            if let Some(config) = eframe::get_value::<Config>(storage, "save_guardian_config") {
                app.config = config.clone();
                app.temp_config = config;
                app.steam_scanner = SteamScanner::new(app.config.steam_path.clone());
                app.non_steam_scanner = NonSteamScanner::new().with_custom_locations(app.config.custom_locations.clone());
                app.backup_manager = BackupManager::new(app.config.backup_path.clone(), app.config.backup_retention_days).ok();
            }
        }

        // Initial scan with forced name refresh
        app.scan_saves();
        app.load_backups();
        
        // Force a secondary name normalization to ensure all displayed names are correct
        app.normalize_all_game_names();
        
        app
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        match self.config.theme {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
            Theme::System => {
                // For now, default to dark theme. In a real app, you'd detect system theme
                ctx.set_visuals(egui::Visuals::dark());
            }
        }
    }

    fn draw_top_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // App title with better styling (no problematic emojis)
            ui.label(egui::RichText::new("Save Guardian").size(20.0).strong().color(egui::Color32::from_rgb(100, 149, 237)));
            
            ui.separator();
            
            // Tab selection with text-based icons to avoid rendering issues
            ui.selectable_value(&mut self.selected_tab, Tab::GameSaves, egui::RichText::new("▶ Game Saves").size(14.0));
            ui.selectable_value(&mut self.selected_tab, Tab::Backups, egui::RichText::new("💾 Backups").size(14.0));
            ui.selectable_value(&mut self.selected_tab, Tab::Sync, egui::RichText::new("⟲ Sync").size(14.0));
            ui.selectable_value(&mut self.selected_tab, Tab::Cloud, egui::RichText::new("☁ Cloud").size(14.0));
            ui.selectable_value(&mut self.selected_tab, Tab::Settings, egui::RichText::new("⚙ Settings").size(14.0));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // About button
                if ui.button(egui::RichText::new("? About").size(12.0)).on_hover_text("About Save Guardian").clicked() {
                    self.show_about = true;
                }
                
                // Quick backup all button
                if ui.button(egui::RichText::new("+ Quick Backup").size(12.0)).on_hover_text("Quick backup all recent saves").clicked() {
                    // TODO: Implement quick backup
                }
                
                // Refresh button with force name update
                if ui.button(egui::RichText::new("↻ Refresh").size(12.0)).on_hover_text("Refresh all data and fix game names").clicked() {
                    // Force refresh incorrect names before scanning
                    self.steam_scanner.refresh_incorrect_names();
                    self.scan_saves();
                    self.load_backups();
                }
            });
        });
    }

    fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match &self.scan_status {
                ScanStatus::Idle => {
                    ui.label("Ready");
                }
                ScanStatus::Scanning => {
                    ui.spinner();
                    ui.label("Scanning for saves...");
                }
                ScanStatus::Complete(msg) => {
                    ui.label(format!("✅ {}", msg));
                }
                ScanStatus::Error(err) => {
                    ui.colored_label(egui::Color32::RED, format!("❌ {}", err));
                }
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Steam: {} | Non-Steam: {} | Backups: {}", 
                    self.steam_saves.len(), 
                    self.non_steam_saves.len(),
                    self.backups.len()
                ));
            });
        });
    }

    fn draw_game_saves_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Search box
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.search_query);
            
            ui.separator();
            
            // Filters
            ui.checkbox(&mut self.filter_steam, "Steam");
            ui.checkbox(&mut self.filter_non_steam, "Non-Steam");
            
            ui.separator();
            
            // Sort options
            ui.label("Sort by:");
            egui::ComboBox::from_id_source("sort_by")
                .selected_text(format!("{:?}", self.sort_by))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sort_by, SortBy::Name, "Name");
                    ui.selectable_value(&mut self.sort_by, SortBy::LastModified, "Last Modified");
                    ui.selectable_value(&mut self.sort_by, SortBy::Size, "Size");
                    ui.selectable_value(&mut self.sort_by, SortBy::Type, "Type");
                });
        });

        ui.separator();
        
        // Toolbar with bulk actions
        ui.horizontal(|ui| {
            ui.label("Bulk Actions:");
            
            if ui.button("💾 Backup All Visible").on_hover_text("Create backups for all visible saves").clicked() {
                // TODO: Implement bulk backup
            }
            
            if ui.button("↗ Export List").on_hover_text("Export save list to file").clicked() {
                // TODO: Implement export
            }
            
            ui.separator();
            
            ui.label(format!("{} saves found", self.get_filtered_saves().len()));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("↻ Rescan").on_hover_text("Refresh save scan and fix game names").clicked() {
                    self.steam_scanner.refresh_incorrect_names();
                    self.scan_saves();
                }
            });
        });
        
        ui.separator();

        // Game saves list
        let mut filtered_saves = self.get_filtered_saves();
        self.sort_saves(&mut filtered_saves);
        
        // Clone saves data to avoid borrowing issues
        let saves_data: Vec<_> = filtered_saves.iter().map(|save| {
            (
                save.save_type.clone(),
                save.display_name(),
                save.format_size(),
                save.last_modified.map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
                save.save_path.clone(),
            )
        }).collect();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("saves_grid")
                .num_columns(6)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.strong("Type");
                    ui.strong("Game");
                    ui.strong("Size");
                    ui.strong("Last Modified");
                    ui.strong("Path");
                    ui.strong("Actions");
                    ui.end_row();

                    for (i, (save_type, display_name, size, last_mod, save_path)) in saves_data.iter().enumerate() {
                        // Type icon with better formatting
                        let type_icon = match save_type {
                            SaveType::Steam => "🔵",
                            SaveType::NonSteam => "🟢",
                        };
                        ui.label(egui::RichText::new(type_icon).size(16.0));

                        // Game name with app ID
                        ui.label(display_name);

                        // Size
                        ui.label(size);

                        // Last modified
                        ui.label(last_mod);

                        // Path (truncated)
                        let path_str = save_path.to_string_lossy();
                        let truncated_path = if path_str.len() > 50 {
                            format!("...{}", &path_str[path_str.len() - 47..])
                        } else {
                            path_str.to_string()
                        };
                        ui.label(truncated_path).on_hover_text(path_str.as_ref());

                        // Actions with more options
                        ui.horizontal(|ui| {
                            if ui.button("💾 Backup").on_hover_text("Create a backup of this save").clicked() {
                                self.selected_game = Some(i);
                                self.show_backup_dialog = true;
                            }
                            
                            if ui.button("▶ Open").on_hover_text("Open save folder in Explorer").clicked() {
                                if save_path.exists() {
                                    let _ = std::process::Command::new("explorer")
                                        .arg(save_path)
                                        .spawn();
                                }
                            }
                            
                            if ui.button("⎘ Copy Path").on_hover_text("Copy save path to clipboard").clicked() {
                                ui.output_mut(|o| o.copied_text = save_path.to_string_lossy().to_string());
                            }
                            
                            if ui.button("i Info").on_hover_text("Show detailed information").clicked() {
                                self.selected_game = Some(i);
                                // TODO: Show info dialog - we'll implement this
                            }
                        });

                        ui.end_row();
                    }
                });
        });
    }

    fn draw_backups_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("💾 Backup Management");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✖ Cleanup Old").clicked() {
                    if let Some(ref backup_manager) = self.backup_manager {
                        match backup_manager.cleanup_old_backups() {
                            Ok(count) => {
                                self.scan_status = ScanStatus::Complete(format!("Cleaned up {} old backups", count));
                                self.load_backups();
                            }
                            Err(e) => {
                                self.scan_status = ScanStatus::Error(format!("Cleanup failed: {}", e));
                            }
                        }
                    }
                }
            });
        });

        // Backup stats
        if let Some(ref stats) = self.backup_stats {
            ui.horizontal(|ui| {
                ui.group(|ui| {
                    ui.label(format!("Total: {}", stats.total_count));
                });
                ui.group(|ui| {
                    ui.label(format!("Steam: {}", stats.steam_count));
                });
                ui.group(|ui| {
                    ui.label(format!("Non-Steam: {}", stats.non_steam_count));
                });
                ui.group(|ui| {
                    ui.label(format!("Size: {}", stats.format_total_size()));
                });
            });
        }

        ui.separator();

        // Backups list
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("backups_grid")
                .num_columns(7)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.strong("Type");
                    ui.strong("Game");
                    ui.strong("Original Location");
                    ui.strong("Created");
                    ui.strong("Size");
                    ui.strong("Description");
                    ui.strong("Actions");
                    ui.end_row();

                    // Store backup actions to avoid borrowing issues
                    let mut folder_to_open: Option<BackupInfo> = None;
                    let mut backup_to_delete: Option<BackupInfo> = None;
                    let mut restore_backup_index: Option<usize> = None;
                    
                    for (i, backup) in self.backups.iter().enumerate() {
                        // Type icon with better formatting
                        let type_icon = match backup.save_type {
                            SaveType::Steam => "🔵",
                            SaveType::NonSteam => "🟢",
                        };
                        ui.label(egui::RichText::new(type_icon).size(16.0));

                        // Game name
                        ui.label(&backup.game_name);

                        // Original location - show the improved path display
                        let original_path_display = backup.display_original_path();
                        if backup.is_cloud_download() {
                            ui.colored_label(egui::Color32::from_rgb(100, 149, 237), original_path_display);
                        } else {
                            // Regular backup - show path with tooltip
                            ui.label(&original_path_display)
                                .on_hover_text(format!("Full path: {}", backup.original_path.display()));
                        }

                        // Created date
                        ui.label(backup.created_at.format("%Y-%m-%d %H:%M").to_string());

                        // Size
                        ui.label(backup.format_size());

                        // Description
                        let desc = backup.description.as_deref().unwrap_or("No description");
                        ui.label(desc);

                        // Actions
                        ui.horizontal(|ui| {
                            // Open backup folder button
                            if ui.button("📂").on_hover_text("Open backup folder in file explorer").clicked() {
                                folder_to_open = Some(backup.clone());
                            }
                            
                            if ui.button("↺").on_hover_text("Restore this backup").clicked() {
                                restore_backup_index = Some(i);
                            }
                            
                            if ui.button("❌").on_hover_text("Delete this backup").clicked() {
                                backup_to_delete = Some(backup.clone());
                            }
                        });

                        ui.end_row();
                    }
                    
                    // Handle actions outside the loop
                    if let Some(backup_info) = folder_to_open {
                        if let Some(ref backup_manager) = self.backup_manager {
                            match backup_manager.open_backup_folder(&backup_info) {
                                Ok(_) => {
                                    self.scan_status = ScanStatus::Complete("Backup folder opened".to_string());
                                }
                                Err(e) => {
                                    self.scan_status = ScanStatus::Error(format!("Failed to open folder: {}", e));
                                }
                            }
                        }
                    }
                    
                    if let Some(index) = restore_backup_index {
                        self.selected_backup = Some(index);
                        self.show_restore_dialog = true;
                    }
                    
                    if let Some(backup_info) = backup_to_delete {
                        if let Some(ref backup_manager) = self.backup_manager {
                            match backup_manager.delete_backup(&backup_info) {
                                Ok(_) => {
                                    self.scan_status = ScanStatus::Complete("Backup deleted".to_string());
                                    self.load_backups();
                                }
                                Err(e) => {
                                    self.scan_status = ScanStatus::Error(format!("Delete failed: {}", e));
                                }
                            }
                        }
                    }
                });
        });
    }

    fn draw_sync_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🔄 Save Synchronization");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⌕ Find Pairs").clicked() {
                    self.sync_pairs = self.sync_manager.find_sync_pairs(&self.steam_saves, &self.non_steam_saves);
                    self.scan_status = ScanStatus::Complete(format!("Found {} sync pairs", self.sync_pairs.len()));
                }
            });
        });

        ui.separator();

        // Sync pairs list
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("sync_grid")
                .num_columns(5)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.strong("Game");
                    ui.strong("Steam Save");
                    ui.strong("Non-Steam Save");
                    ui.strong("Last Synced");
                    ui.strong("Actions");
                    ui.end_row();

                    for (i, pair) in self.sync_pairs.iter().enumerate() {
                        // Game name
                        ui.label(&pair.game_name);

                        // Steam save status with better icons
                        match &pair.steam_save {
                            Some(_save) => {
                                ui.colored_label(egui::Color32::from_rgb(46, 204, 64), "🔵 Available");
                            }
                            None => {
                                ui.colored_label(egui::Color32::from_rgb(255, 133, 27), "⚫ Missing");
                            }
                        }

                        // Non-Steam save status with better icons
                        match &pair.non_steam_save {
                            Some(_save) => {
                                ui.colored_label(egui::Color32::from_rgb(46, 204, 64), "🟢 Available");
                            }
                            None => {
                                ui.colored_label(egui::Color32::from_rgb(255, 133, 27), "⚫ Missing");
                            }
                        }

                        // Last synced
                        let last_sync = pair.last_synced
                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "Never".to_string());
                        ui.label(last_sync);

                        // Actions
                        ui.horizontal(|ui| {
                            if pair.steam_save.is_some() && pair.non_steam_save.is_some() {
                                if ui.button("⟲ Sync").clicked() {
                                    self.selected_sync_pair = Some(i);
                                    self.show_sync_dialog = true;
                                }
                            }
                            
                            if pair.steam_save.is_some() && pair.non_steam_save.is_none() {
                                ui.colored_label(egui::Color32::YELLOW, "Need non-Steam location");
                            }
                            
                            if pair.non_steam_save.is_some() && pair.steam_save.is_none() {
                                ui.colored_label(egui::Color32::YELLOW, "Need Steam location");
                            }
                        });

                        ui.end_row();
                    }
                });
        });

        // Display last sync result if available
        if let Some(ref result) = self.last_sync_result {
            ui.separator();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Last Sync Result:");
                    ui.label(format!("✅ {} files copied ({})", result.files_copied, result.format_bytes_copied()));
                    ui.label(format!("at {}", result.sync_time.format("%H:%M:%S")));
                });
            });
        }
    }

    fn draw_cloud_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("☁ Koofr Cloud Sync");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status_color = if self.config.koofr_config.enabled {
                    egui::Color32::from_rgb(46, 204, 64)
                } else {
                    egui::Color32::from_rgb(255, 133, 27)
                };
                let status_text = if self.config.koofr_config.enabled { "Enabled" } else { "Disabled" };
                ui.colored_label(status_color, status_text);
            });
        });
        
        ui.separator();
        
        if !self.config.koofr_config.enabled {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("Koofr cloud sync is disabled").size(16.0));
                ui.add_space(10.0);
                ui.label("Configure your Koofr credentials in Settings to enable cloud backup.");
                ui.add_space(20.0);
                if ui.button(egui::RichText::new("⚙ Go to Settings").size(14.0)).clicked() {
                    self.selected_tab = Tab::Settings;
                }
                
                ui.add_space(20.0);
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.strong("Koofr Setup Instructions:");
                        ui.label("1. Create account at https://app.koofr.net");
                        ui.label("2. Generate app password in account settings");
                        ui.label("3. Use WebDAV URL: https://app.koofr.net/dav/Koofr");
                        ui.label("4. Enter your email and app password in Settings");
                    });
                });
            });
            return;
        }
        
        // Cloud sync status and controls
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.strong("Connection Status");
                    ui.colored_label(egui::Color32::from_rgb(46, 204, 64), "✓ Connected");
                    ui.label(format!("Server: {}", self.config.koofr_config.server_url));
                    ui.label(format!("User: {}", self.config.koofr_config.username));
                });
            });
            
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.strong("Sync Statistics");
                    let last_sync_text = match self.last_sync_time {
                        Some(time) => time.format("%Y-%m-%d %H:%M UTC").to_string(),
                        None => "Never".to_string(),
                    };
                    ui.label(format!("Last sync: {}", last_sync_text));
                    ui.label(format!("Files synced: {}", self.cloud_files_synced));
                    let storage_mb = self.cloud_storage_used as f64 / (1024.0 * 1024.0);
                    ui.label(format!("Cloud storage used: {:.1} MB", storage_mb));
                });
            });
        });
        
        ui.separator();
        
        // Manual sync controls
        ui.horizontal(|ui| {
            ui.label("Manual Sync:");
            
            if ui.button("↑ Upload All Backups").on_hover_text("Upload all local backups to cloud").clicked() {
                self.upload_backups_to_koofr();
            }
            
            if ui.button("↓ Download from Cloud").on_hover_text("Download backups from cloud").clicked() {
                self.download_backups_from_koofr();
            }
            
            if ui.button("⟲ Full Sync").on_hover_text("Synchronize local and cloud backups").clicked() {
                self.full_sync_koofr();
            }
        });
        
        ui.separator();
        
        // Cloud backup list
        ui.strong("Cloud Backups");
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.label("No cloud backups found.");
                ui.label("Upload some backups to see them here.");
            });
            
            // TODO: Display actual cloud backup list
            // This would show backups stored in Koofr with download/delete options
        });
    }

    fn draw_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙️ Settings");

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.strong("Paths");
                ui.separator();
                
                ui.horizontal(|ui| {
                    ui.label("Steam userdata path:");
                    ui.text_edit_singleline(&mut self.temp_config.steam_path.to_string_lossy().to_string());
                    if ui.button("📁 Browse").clicked() {
                        // TODO: Open file dialog
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Backup directory:");
                    ui.text_edit_singleline(&mut self.temp_config.backup_path.to_string_lossy().to_string());
                    if ui.button("📁 Browse").clicked() {
                        // TODO: Open file dialog
                    }
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.strong("Backup Settings");
                ui.separator();
                
                ui.checkbox(&mut self.temp_config.auto_backup, "Automatically backup saves before operations");
                
                ui.horizontal(|ui| {
                    ui.label("Keep backups for");
                    ui.add(egui::DragValue::new(&mut self.temp_config.backup_retention_days).clamp_range(1..=365).suffix(" days"));
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.strong("Cloud Sync (Koofr)");
                ui.separator();
                
                ui.checkbox(&mut self.temp_config.koofr_config.enabled, "Enable Koofr cloud sync");
                
                ui.horizontal(|ui| {
                    ui.label("Server URL:");
                    ui.text_edit_singleline(&mut self.temp_config.koofr_config.server_url);
                });
                ui.label(egui::RichText::new("Use: https://app.koofr.net/dav/Koofr").size(11.0).color(egui::Color32::GRAY));
                
                ui.horizontal(|ui| {
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.temp_config.koofr_config.username);
                });
                ui.label(egui::RichText::new("Your Koofr email address").size(11.0).color(egui::Color32::GRAY));
                
                ui.horizontal(|ui| {
                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.temp_config.koofr_config.password).password(true));
                });
                ui.label(egui::RichText::new("Generate app password at: Account Settings > Passwords").size(11.0).color(egui::Color32::GRAY));
                
                ui.horizontal(|ui| {
                    ui.label("Sync Folder:");
                    ui.text_edit_singleline(&mut self.temp_config.koofr_config.sync_folder);
                });
                
                ui.checkbox(&mut self.temp_config.koofr_config.auto_sync, "Automatic sync");
                
                ui.horizontal(|ui| {
                    ui.label("Sync interval:");
                    ui.add(egui::Slider::new(&mut self.temp_config.koofr_config.sync_interval_minutes, 5..=1440).text("minutes"));
                });
                
                if ui.button("✓ Test Connection").on_hover_text("Test Koofr connection").clicked() {
                    self.test_koofr_connection();
                }
            });
            
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.strong("Scan Settings");
                ui.separator();
                
                ui.checkbox(&mut self.temp_config.auto_backup, "Enable automatic scanning on startup");
                
                ui.horizontal(|ui| {
                    ui.label("Scan depth:");
                    ui.add(egui::Slider::new(&mut self.temp_config.backup_retention_days, 1..=7).text("levels").clamp_to_range(true));
                });
                
                ui.checkbox(&mut self.temp_config.auto_backup, "Include system locations in scan");
                ui.checkbox(&mut self.temp_config.auto_backup, "Detect saves by content analysis");
            });
            
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.strong("Appearance");
                ui.separator();
                
                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    egui::ComboBox::from_id_source("theme_combo")
                        .selected_text(format!("{:?}", self.temp_config.theme))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.temp_config.theme, Theme::Dark, "🌑 Dark");
                            ui.selectable_value(&mut self.temp_config.theme, Theme::Light, "☀️ Light");
                            ui.selectable_value(&mut self.temp_config.theme, Theme::System, "⚙️ System");
                        });
                });
                
                ui.checkbox(&mut self.temp_config.auto_backup, "Show detailed file information");
                ui.checkbox(&mut self.temp_config.auto_backup, "Enable advanced tooltips");
                ui.checkbox(&mut self.temp_config.auto_backup, "Show confirmation dialogs");
            });
            
            ui.add_space(10.0);
            
            ui.group(|ui| {
                ui.strong("Advanced Options");
                ui.separator();
                
                ui.checkbox(&mut self.temp_config.auto_backup, "Enable logging");
                ui.checkbox(&mut self.temp_config.auto_backup, "Monitor saves for changes");
                ui.checkbox(&mut self.temp_config.auto_backup, "Enable cloud sync preparation");
                
                if ui.button("✖ Clear All Cache").on_hover_text("Clear application cache and temporary files").clicked() {
                    // TODO: Implement cache clearing
                }
                
                if ui.button("↺ Reset to Defaults").on_hover_text("Reset all settings to default values").clicked() {
                    self.temp_config = Config::default();
                }
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if ui.button("✓ Save Settings").clicked() {
                    self.config = self.temp_config.clone();
                    self.steam_scanner = SteamScanner::new(self.config.steam_path.clone());
                    self.non_steam_scanner = NonSteamScanner::new().with_custom_locations(self.config.custom_locations.clone());
                    self.backup_manager = BackupManager::new(self.config.backup_path.clone(), self.config.backup_retention_days).ok();
                    self.scan_status = ScanStatus::Complete("Settings saved successfully!".to_string());
                }
                
                if ui.button("↺ Reset to Default").clicked() {
                    self.temp_config = Config::default();
                }
            });
        });
    }

    fn draw_modals(&mut self, ctx: &egui::Context) {
        // About dialog
        if self.show_about {
            egui::Window::new("About Save Guardian")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("🛡️ Save Guardian");
                        ui.label("Version 1.0.0");
                        ui.add_space(10.0);
                        ui.label("A sleek save manager for Steam and non-Steam games");
                        ui.add_space(10.0);
                        ui.label("Features:");
                        ui.label("• Automatic save detection");
                        ui.label("• Backup and restore");
                        ui.label("• Save synchronization");
                        ui.label("• Modern, intuitive interface");
                        ui.add_space(10.0);
                        if ui.button("Close").clicked() {
                            self.show_about = false;
                        }
                    });
                });
        }

        // Backup dialog
        if self.show_backup_dialog {
            if let Some(game_idx) = self.selected_game {
                let saves = self.get_filtered_saves();
                if let Some(save) = saves.get(game_idx) {
                    // Clone the save data to avoid borrowing issues
                    let save_name = save.name.clone();
                    let save_path = save.save_path.clone();
                    let save_size = save.format_size();
                    let save_clone = (*save).clone();
                    
                    egui::Window::new(format!("Backup {}", save_name))
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label(format!("Create backup of: {}", save_name));
                            ui.label(format!("Path: {}", save_path.display()));
                            ui.label(format!("Size: {}", save_size));
                            
                            ui.add_space(10.0);
                            
                            // We need to persist the description across frames
                            let mut description = String::new();
                            ui.horizontal(|ui| {
                                ui.label("Description:");
                                ui.text_edit_singleline(&mut description);
                            });
                            
                            ui.add_space(10.0);
                            
                            ui.horizontal(|ui| {
                                if ui.button("💾 Create Backup").clicked() {
                                    if let Some(ref backup_manager) = self.backup_manager {
                                        match backup_manager.create_backup(
                                            &save_clone,
                                            if description.is_empty() { None } else { Some(description) }
                                        ) {
                                            Ok(_) => {
                                                self.scan_status = ScanStatus::Complete("Backup created successfully".to_string());
                                                self.load_backups();
                                            }
                                            Err(e) => {
                                                self.scan_status = ScanStatus::Error(format!("Backup failed: {}", e));
                                            }
                                        }
                                    }
                                    self.show_backup_dialog = false;
                                }
                                
                                if ui.button("Cancel").clicked() {
                                    self.show_backup_dialog = false;
                                }
                            });
                        });
                }
            }
        }
        
        // Additional dialogs would go here...
    }

    // Helper methods
    fn scan_saves(&mut self) {
        self.scan_status = ScanStatus::Scanning;
        
        // Don't pre-load hardcoded database - let the API fetching work dynamically
        // self.steam_scanner.load_game_database();
        
        // Refresh any incorrect cached names before scanning
        self.steam_scanner.refresh_incorrect_names();
        
        // Scan Steam saves
        match self.steam_scanner.scan_steam_saves() {
            Ok(users) => {
                self.steam_saves.clear();
                let mut seen_games: std::collections::HashMap<u32, GameSave> = std::collections::HashMap::new();
                
                for user in users {
                    for game in user.games {
                        // Use app_id as the key for deduplication
                        if let Some(app_id) = game.app_id {
                            // Keep the most recent version of the game (by last_modified)
                            let should_add = match seen_games.get(&app_id) {
                                Some(existing_game) => {
                                    match (game.last_modified, existing_game.last_modified) {
                                        (Some(new_time), Some(existing_time)) => new_time > existing_time,
                                        (Some(_), None) => true,
                                        _ => false,
                                    }
                                }
                                None => true,
                            };
                            
                            if should_add {
                                seen_games.insert(app_id, game.clone());
                            }
                        } else {
                            // For games without app_id, add them all (shouldn't happen for Steam games)
                            self.steam_saves.push(game);
                        }
                    }
                }
                
                // Add all the deduplicated games
                for (_, game) in seen_games {
                    self.steam_saves.push(game);
                }

                // Normalize names after scan using the refreshed cache so UI shows correct names
                for save in &mut self.steam_saves {
                    if let Some(app_id) = save.app_id {
                        // Re-fetch name through the scanner which now prefers correct API names
                        let fixed_name = self.steam_scanner.get_game_name(app_id);
                        save.name = fixed_name;
                    }
                }
                
                info!("After deduplication: {} unique Steam games", self.steam_saves.len());
            }
            Err(e) => {
                error!("Failed to scan Steam saves: {}", e);
            }
        }
        
        // Scan non-Steam saves
        match self.non_steam_scanner.scan_non_steam_saves() {
            Ok(saves) => {
                self.non_steam_saves = saves;
            }
            Err(e) => {
                error!("Failed to scan non-Steam saves: {}", e);
            }
        }
        
        self.scan_status = ScanStatus::Complete(format!(
            "Found {} Steam saves and {} non-Steam saves",
            self.steam_saves.len(),
            self.non_steam_saves.len()
        ));
        
        info!("Scan complete: {} Steam, {} non-Steam", self.steam_saves.len(), self.non_steam_saves.len());
        
        // Always normalize names after any scan to ensure UI consistency
        self.normalize_all_game_names();
    }
    
    /// Force normalize all Steam game names using the current cache
    fn normalize_all_game_names(&mut self) {
        for save in &mut self.steam_saves {
            if let Some(app_id) = save.app_id {
                let correct_name = self.steam_scanner.get_game_name(app_id);
                if save.name != correct_name {
                    info!("Normalizing game name: '{}' -> '{}' for app {}", save.name, correct_name, app_id);
                    save.name = correct_name;
                }
            }
        }
    }
    
    fn load_backups(&mut self) {
        if let Some(ref backup_manager) = self.backup_manager {
            match backup_manager.list_backups(None, None) {
                Ok(backups) => {
                    self.backups = backups;
                }
                Err(e) => {
                    error!("Failed to load backups: {}", e);
                }
            }
            
            match backup_manager.get_backup_stats() {
                Ok(stats) => {
                    self.backup_stats = Some(stats);
                }
                Err(e) => {
                    error!("Failed to get backup stats: {}", e);
                }
            }
        }
    }
    
    fn get_filtered_saves(&self) -> Vec<&GameSave> {
        let mut saves = Vec::new();
        
        if self.filter_steam {
            saves.extend(self.steam_saves.iter());
        }
        
        if self.filter_non_steam {
            saves.extend(self.non_steam_saves.iter());
        }
        
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            saves.retain(|save| {
                // Use the same display string as in the UI so results are consistent
                let display = save.display_name().to_lowercase();
                display.contains(&query) ||
                save.save_path.to_string_lossy().to_lowercase().contains(&query)
            });
        }
        
        saves
    }
    
    fn sort_saves(&self, saves: &mut Vec<&GameSave>) {
        match self.sort_by {
            SortBy::Name => saves.sort_by(|a, b| a.name.cmp(&b.name)),
            SortBy::LastModified => saves.sort_by(|a, b| b.last_modified.cmp(&a.last_modified)),
            SortBy::Size => saves.sort_by(|a, b| b.size.cmp(&a.size)),
            SortBy::Type => saves.sort_by(|a, b| a.save_type.cmp(&b.save_type)),
        }
    }
    
    fn initialize_cloud_folder(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::new();
        let sync_folder_path = format!("{}/{}", 
            self.config.koofr_config.server_url.trim_end_matches('/'),
            self.config.koofr_config.sync_folder.trim_start_matches('/')
        );
        
        info!("Attempting to create cloud folder at: {}", sync_folder_path);
        
        let response = client
            .request(reqwest::Method::from_bytes(b"MKCOL").map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?, &sync_folder_path)
            .basic_auth(&self.config.koofr_config.username, Some(&self.config.koofr_config.password))
            .timeout(std::time::Duration::from_secs(30))
            .send()?;
        
        match response.status() {
            reqwest::StatusCode::METHOD_NOT_ALLOWED => {
                info!("Cloud folder already exists (405 Method Not Allowed)");
                Ok(())
            },
            reqwest::StatusCode::CREATED => {
                info!("Cloud folder created successfully (201 Created)");
                Ok(())
            },
            reqwest::StatusCode::NOT_FOUND => {
                error!("Parent directory doesn't exist (404 Not Found)");
                Err("Parent directory doesn't exist in cloud storage".into())
            },
            status => {
                warn!("Unexpected response when creating folder: {}", status);
                if status.is_success() {
                    Ok(())
                } else {
                    Err(format!("Failed to create folder: HTTP {}", status).into())
                }
            }
        }
    }
    
    fn test_koofr_connection(&mut self) {
        let koofr_config = &self.temp_config.koofr_config;
        
        if koofr_config.server_url.is_empty() || koofr_config.username.is_empty() || koofr_config.password.is_empty() {
            self.scan_status = ScanStatus::Error("Please fill in all Koofr connection details".to_string());
            return;
        }
        
        self.scan_status = ScanStatus::Scanning;
        
        // Test the WebDAV connection
        let client = reqwest::blocking::Client::new();
        let test_url = format!("{}/", koofr_config.server_url.trim_end_matches('/'));
        
        match client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &test_url)
            .basic_auth(&koofr_config.username, Some(&koofr_config.password))
            .header("Depth", "0")
            .timeout(std::time::Duration::from_secs(10))
            .send()
        {
            Ok(response) => {
                if response.status().is_success() {
                    self.scan_status = ScanStatus::Complete("✓ Koofr connection successful!".to_string());
                } else {
                    self.scan_status = ScanStatus::Error(format!(
                        "Koofr connection failed: HTTP {}", 
                        response.status().as_u16()
                    ));
                }
            }
            Err(e) => {
                self.scan_status = ScanStatus::Error(format!(
                    "Koofr connection error: {}", 
                    e.to_string()
                ));
            }
        }
    }
    
    fn upload_backups_to_koofr(&mut self) {
        if !self.config.koofr_config.enabled {
            self.scan_status = ScanStatus::Error("Koofr sync is not enabled".to_string());
            return;
        }
        
        // Refresh backups list before uploading
        self.load_backups();
        
        info!("Found {} backups to potentially upload", self.backups.len());
        
        // Log backup directory contents for debugging
        if let Some(ref backup_manager) = self.backup_manager {
            // Get backup directory from config
            let backup_dir = &self.config.backup_path;
            info!("Backup directory: {}", backup_dir.display());
            
            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                let zip_files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "zip"))
                    .collect();
                info!("Found {} ZIP files in backup directory", zip_files.len());
                
                for entry in zip_files.iter().take(5) { // Log first 5 files
                    info!("Backup file: {}", entry.path().display());
                }
            }
        }
        
        if self.backups.is_empty() {
            self.scan_status = ScanStatus::Error("No backups found. Create some backups first!".to_string());
            return;
        }
        
        self.scan_status = ScanStatus::Scanning;
        
        // Clone config to avoid borrowing issues
        let koofr_config = self.config.koofr_config.clone();
        
        let client = reqwest::blocking::Client::new();
        let mut uploaded_count = 0;
        let mut total_size = 0u64;
        
        // Initialize cloud folder first
        match self.initialize_cloud_folder() {
            Ok(()) => {
                info!("Cloud folder is ready for upload");
            },
            Err(e) => {
                warn!("Could not initialize cloud folder: {}", e);
                // Continue anyway - might already exist or be accessible
            }
        }
        
        // Upload each backup
        for (i, backup) in self.backups.iter().enumerate() {
            info!("Processing backup {}: {}", i + 1, backup.backup_path.display());
            
            if backup.backup_path.exists() {
                let filename = backup.backup_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("backup.zip");
                
                let upload_url = format!("{}/{}/{}", 
                    koofr_config.server_url.trim_end_matches('/'),
                    koofr_config.sync_folder.trim_start_matches('/'),
                    filename
                );
                
                info!("Uploading {} to {}", filename, upload_url);
                
                match std::fs::read(&backup.backup_path) {
                    Ok(file_data) => {
                        info!("Read {} bytes from {}", file_data.len(), filename);
                        
                        match client
                            .put(&upload_url)
                            .basic_auth(&koofr_config.username, Some(&koofr_config.password))
                            .header("Content-Type", "application/zip")
                            .body(file_data.clone())
                            .timeout(std::time::Duration::from_secs(60))
                            .send()
                        {
                            Ok(response) => {
                                let status = response.status();
                                info!("Upload response for {}: HTTP {}", filename, status);
                                
                                if status.is_success() {
                                    uploaded_count += 1;
                                    total_size += file_data.len() as u64;
                                    info!("Successfully uploaded {}", filename);
                                } else {
                                    let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
                                    warn!("Failed to upload {}: HTTP {} - {}", filename, status, error_text);
                                }
                            }
                            Err(e) => {
                                warn!("Upload error for {}: {}", filename, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read backup file {}: {}", backup.backup_path.display(), e);
                    }
                }
            } else {
                warn!("Backup file does not exist: {}", backup.backup_path.display());
            }
        }
        
        if uploaded_count > 0 {
            // Update sync statistics
            self.last_sync_time = Some(chrono::Utc::now());
            self.cloud_files_synced = uploaded_count;
            self.cloud_storage_used = total_size;
            
            self.scan_status = ScanStatus::Complete(format!(
                "✓ Uploaded {} backups ({:.1} MB) to Koofr", 
                uploaded_count, 
                total_size as f64 / (1024.0 * 1024.0)
            ));
        } else {
            self.scan_status = ScanStatus::Error("No backups were uploaded".to_string());
        }
    }
    
    fn download_backups_from_koofr(&mut self) {
        if !self.config.koofr_config.enabled {
            self.scan_status = ScanStatus::Error("Koofr sync is not enabled".to_string());
            return;
        }
        
        self.scan_status = ScanStatus::Scanning;
        
        // Clone config to avoid borrowing issues
        let koofr_config = self.config.koofr_config.clone();
        let backup_path = self.config.backup_path.clone();
        
        let client = reqwest::blocking::Client::new();
        let folder_url = format!("{}/{}/", 
            koofr_config.server_url.trim_end_matches('/'),
            koofr_config.sync_folder.trim_start_matches('/')
        );
        
        info!("Downloading from cloud folder: {}", folder_url);
        info!("Download destination: {}", backup_path.display());
        
        // Ensure backup directory exists
        if let Err(e) = std::fs::create_dir_all(&backup_path) {
            self.scan_status = ScanStatus::Error(format!("Failed to create backup directory: {}", e));
            return;
        }
        
        // Initialize cloud folder first
        match self.initialize_cloud_folder() {
            Ok(()) => {
                info!("Cloud folder is ready for download");
            },
            Err(e) => {
                warn!("Could not initialize cloud folder for download: {}", e);
                // Continue anyway - might already exist
            }
        }
        
        // List files in the cloud folder using PROPFIND
        let propfind_body = r#"<?xml version="1.0" encoding="utf-8" ?>
        <D:propfind xmlns:D="DAV:">
            <D:prop>
                <D:displayname/>
                <D:getcontentlength/>
            </D:prop>
        </D:propfind>"#;
        
        match client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &folder_url)
            .basic_auth(&koofr_config.username, Some(&koofr_config.password))
            .header("Depth", "1")
            .header("Content-Type", "text/xml")
            .body(propfind_body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
        {
            Ok(response) => {
                info!("PROPFIND response: {}", response.status());
                
                if response.status().is_success() {
                    let response_text = response.text().unwrap_or_else(|_| "No response body".to_string());
                    info!("Cloud folder contents (first 1000 chars): {}", 
                        if response_text.len() > 1000 { &response_text[..1000] } else { &response_text });
                    
                    // Parse the XML response to extract file names
                    let file_urls = self.extract_file_urls_from_webdav_response(&response_text, &koofr_config);
                    info!("Found {} files to download", file_urls.len());
                    
                    if file_urls.is_empty() {
                        self.scan_status = ScanStatus::Complete("No files found in cloud folder to download".to_string());
                        return;
                    }
                    
                    // Download each file
                    let mut downloaded_count = 0;
                    let mut total_size = 0u64;
                    
                    for (filename, file_url) in &file_urls {
                        info!("Downloading file: {} from {}", filename, file_url);
                        
                        match client
                            .get(file_url)
                            .basic_auth(&koofr_config.username, Some(&koofr_config.password))
                            .timeout(std::time::Duration::from_secs(60))
                            .send()
                        {
                            Ok(file_response) => {
                                if file_response.status().is_success() {
                                    match file_response.bytes() {
                                        Ok(file_data) => {
                                            let local_file_path = backup_path.join(filename);
                                            
                                            match std::fs::write(&local_file_path, &file_data) {
                                                Ok(()) => {
                                                    downloaded_count += 1;
                                                    total_size += file_data.len() as u64;
                                                    info!("Successfully downloaded {} ({} bytes) to {}", 
                                                        filename, file_data.len(), local_file_path.display());
                                                    
                                                    // Create metadata for the downloaded backup so it appears in the Backups tab
                                                    self.create_metadata_for_downloaded_backup(filename, &local_file_path, file_data.len() as u64);
                                                },
                                                Err(e) => {
                                                    warn!("Failed to write downloaded file {}: {}", filename, e);
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            warn!("Failed to read response data for {}: {}", filename, e);
                                        }
                                    }
                                } else {
                                    warn!("Failed to download {}: HTTP {}", filename, file_response.status());
                                }
                            },
                            Err(e) => {
                                warn!("Download error for {}: {}", filename, e);
                            }
                        }
                    }
                    
                    // Update status and statistics
                    if downloaded_count > 0 {
                        // Update sync statistics
                        self.last_sync_time = Some(chrono::Utc::now());
                        self.cloud_files_synced = downloaded_count;
                        self.cloud_storage_used = total_size;
                        
                        // Refresh backups list to show the downloaded files
                        self.load_backups();
                        
                        self.scan_status = ScanStatus::Complete(format!(
                            "✓ Downloaded {} backup files ({:.1} MB) from cloud", 
                            downloaded_count,
                            total_size as f64 / (1024.0 * 1024.0)
                        ));
                    } else {
                        self.scan_status = ScanStatus::Error("No files were downloaded successfully".to_string());
                    }
                    
                } else if response.status().as_u16() == 404 {
                    self.scan_status = ScanStatus::Error("Cloud sync folder not found. Try uploading some backups first.".to_string());
                } else {
                    self.scan_status = ScanStatus::Error(format!(
                        "Failed to list cloud files: HTTP {}", 
                        response.status().as_u16()
                    ));
                }
            }
            Err(e) => {
                self.scan_status = ScanStatus::Error(format!("Cloud connection error: {}", e));
            }
        }
    }
    
    fn extract_file_urls_from_webdav_response(&self, response_text: &str, koofr_config: &KoofrConfig) -> Vec<(String, String)> {
        let mut file_urls = Vec::new();
        
        info!("Starting XML parsing for WebDAV response");
        
        // Parse all <D:href> elements that contain .zip files
        let mut search_pos = 0;
        
        while let Some(start) = response_text[search_pos..].find("<D:href>") {
            let absolute_start = search_pos + start;
            let href_start = absolute_start + 8; // Skip "<D:href>"
            
            if let Some(end_pos) = response_text[href_start..].find("</D:href>") {
                let href_content = &response_text[href_start..href_start + end_pos];
                info!("Found href: {}", href_content);
                
                // Check if this href contains a .zip file
                if (href_content.contains(".zip") || href_content.contains("%2Ezip")) && !href_content.ends_with("/SaveGuardian") {
                    info!("Processing ZIP file href: {}", href_content);
                    
                    // Skip the folder itself
                    if href_content.ends_with("/SaveGuardian") || href_content.ends_with("/SaveGuardian/") {
                        info!("Skipping folder entry: {}", href_content);
                    } else {
                        // Extract just the filename from the full path
                        if let Some(filename_start) = href_content.rfind('/') {
                            let encoded_filename = &href_content[filename_start + 1..];
                            info!("Encoded filename: {}", encoded_filename);
                            
                            // URL decode the filename
                            let filename = self.url_decode(encoded_filename);
                            info!("Decoded filename: {}", filename);
                            
                            if filename.ends_with(".zip") && !filename.is_empty() {
                                // Construct the full download URL
                                // The href_content already starts with /dav/Koofr, so we just need the base URL
                                let base_url = koofr_config.server_url.trim_end_matches('/');
                                let base_url = if base_url.ends_with("/dav/Koofr") {
                                    &base_url[..base_url.len() - 10] // Remove "/dav/Koofr"
                                } else {
                                    base_url
                                };
                                let full_url = format!("{}{}", base_url, href_content);
                                
                                info!("Found file: {} -> {}", filename, full_url);
                                file_urls.push((filename, full_url));
                            } else {
                                info!("Filename doesn't end with .zip or is empty: {}", filename);
                            }
                        } else {
                            info!("No filename found in href: {}", href_content);
                        }
                    }
                } else {
                    info!("Href doesn't contain .zip or is folder: {}", href_content);
                }
                
                search_pos = href_start + end_pos + 9; // Move past </D:href>
            } else {
                info!("No closing </D:href> found after position {}", absolute_start);
                break;
            }
        }
        
        info!("XML parsing complete. Found {} files", file_urls.len());
        file_urls
    }
    
    fn url_decode(&self, encoded: &str) -> String {
        // Simple URL decoding for common cases
        encoded
            .replace("%20", " ")
            .replace("%28", "(")
            .replace("%29", ")")
            .replace("%2E", ".")
            .replace("%2F", "/")
            .replace("%3A", ":")
            .replace("%5F", "_")
            .replace("%2D", "-")
    }
    
    fn create_metadata_for_downloaded_backup(&self, filename: &str, backup_path: &std::path::PathBuf, size: u64) {
        use crate::types::*;
        use std::path::PathBuf;
        
        // Extract information from filename
        // Format: GameName_AppID_SaveType_Timestamp.zip
        let backup_id = filename.strip_suffix(".zip").unwrap_or(filename);
        
        // First, try to find if we have a local copy of this backup's metadata already
        // This happens when we previously uploaded this backup and still have the local copy
        if let Some(ref backup_manager) = self.backup_manager {
            // Look for existing metadata with the same base ID (without timestamp)
            let base_id = self.extract_base_backup_id(backup_id);
            info!("Looking for existing metadata for base ID: {}", base_id);
            
            // Try to find a similar backup in our current backups
            match backup_manager.list_backups(None, None) {
                Ok(existing_backups) => {
                    for existing_backup in existing_backups {
                        let existing_base_id = self.extract_base_backup_id(&existing_backup.id);
                        if existing_base_id == base_id {
                            info!("Found matching local backup metadata for {}", base_id);
                            
                            // Use the original backup's information but mark it as downloaded
                            let backup_info = BackupInfo {
                                id: backup_id.to_string(),
                                game_name: existing_backup.game_name.clone(),
                                app_id: existing_backup.app_id,
                                save_type: existing_backup.save_type.clone(),
                                original_path: existing_backup.original_path.clone(), // Use the REAL original path!
                                backup_path: backup_path.clone(),
                                created_at: chrono::Utc::now(),
                                size,
                                description: Some(format!("📥 Downloaded from cloud - Original: {}", existing_backup.original_path.display())),
                            };
                            
                            self.save_backup_metadata_directly(&backup_info);
                            return;
                        }
                    }
                }
                Err(_) => {}
            }
        }
        
        // If we didn't find existing metadata, fall back to parsing the filename
        info!("No existing metadata found, parsing filename: {}", filename);
        
        // Parse filename to extract game info
        let parts: Vec<&str> = backup_id.split('_').collect();
        let (game_name, app_id, save_type, original_path) = if parts.len() >= 3 {
            let save_type_part = parts[parts.len() - 2]; // second to last should be save type
            let save_type = if save_type_part == "steam" { SaveType::Steam } else { SaveType::NonSteam };
            
            // Try to extract app_id if it's a number
            let mut app_id = None;
            let mut name_parts = Vec::new();
            
            for (i, part) in parts.iter().enumerate() {
                if i == parts.len() - 1 { // skip timestamp
                    continue;
                }
                if i == parts.len() - 2 { // skip save type
                    continue;
                }
                
                // Check if this part looks like an app ID (numeric)
                if let Ok(id) = part.parse::<u32>() {
                    app_id = Some(id);
                } else {
                    name_parts.push(*part);
                }
            }
            
            let game_name = if name_parts.is_empty() {
                "Downloaded Game".to_string()
            } else {
                name_parts.join(" ").replace('_', " ")
            };
            
            // Try to find the actual save path from current scanned saves
            let actual_original_path = self.find_actual_save_path(&game_name, app_id, &save_type)
                .unwrap_or_else(|| self.reconstruct_likely_original_path(&game_name, app_id, &save_type));
            
            (game_name, app_id, save_type, actual_original_path)
        } else {
            let fallback_path = PathBuf::from("📥 Downloaded from Cloud Storage");
            ("Downloaded Game".to_string(), None, SaveType::NonSteam, fallback_path)
        };
        
        // Create backup info
        let backup_info = BackupInfo {
            id: backup_id.to_string(),
            game_name: game_name.clone(),
            app_id,
            save_type: save_type.clone(),
            original_path,
            backup_path: backup_path.clone(),
            created_at: chrono::Utc::now(),
            size,
            description: Some(format!("📥 Downloaded from cloud storage - {}", game_name)),
        };
        
        self.save_backup_metadata_directly(&backup_info);
    }
    
    /// Extract base backup ID without timestamp
    fn extract_base_backup_id(&self, full_id: &str) -> String {
        // Remove the timestamp part (last part after the final underscore)
        // Format: GameName_AppID_SaveType_Timestamp -> GameName_AppID_SaveType
        let parts: Vec<&str> = full_id.split('_').collect();
        if parts.len() > 1 {
            // Check if the last part looks like a timestamp (8 or 14 digits)
            if let Some(last_part) = parts.last() {
                if last_part.len() >= 8 && last_part.chars().all(|c| c.is_ascii_digit()) {
                    // Remove timestamp part
                    parts[..parts.len()-1].join("_")
                } else {
                    full_id.to_string()
                }
            } else {
                full_id.to_string()
            }
        } else {
            full_id.to_string()
        }
    }
    
    /// Find actual save path from currently scanned saves
    fn find_actual_save_path(&self, game_name: &str, app_id: Option<u32>, save_type: &SaveType) -> Option<std::path::PathBuf> {
        match save_type {
            SaveType::Steam => {
                // Look through Steam saves for matching game
                for save in &self.steam_saves {
                    if let Some(id) = app_id {
                        if save.app_id == Some(id) {
                            info!("Found actual Steam save path for app ID {}: {}", id, save.save_path.display());
                            return Some(save.save_path.clone());
                        }
                    }
                    
                    // Also try name matching as fallback
                    if save.name.to_lowercase().contains(&game_name.to_lowercase()) {
                        info!("Found Steam save path by name match '{}': {}", game_name, save.save_path.display());
                        return Some(save.save_path.clone());
                    }
                }
            },
            SaveType::NonSteam => {
                // Look through non-Steam saves for matching game
                for save in &self.non_steam_saves {
                    if save.name.to_lowercase().contains(&game_name.to_lowercase()) ||
                       game_name.to_lowercase().contains(&save.name.to_lowercase()) {
                        info!("Found actual non-Steam save path for '{}': {}", game_name, save.save_path.display());
                        return Some(save.save_path.clone());
                    }
                }
            }
        }
        
        None
    }
    
    /// Save backup metadata directly to file
    fn save_backup_metadata_directly(&self, backup_info: &BackupInfo) {
        let metadata_path = self.config.backup_path.join(format!("{}.backup.json", backup_info.id));
        
        if let Ok(metadata_json) = serde_json::to_string_pretty(backup_info) {
            if let Err(e) = std::fs::write(&metadata_path, metadata_json) {
                warn!("Failed to create metadata for downloaded backup {}: {}", backup_info.id, e);
            } else {
                info!("Created metadata for downloaded backup: {} -> {}", metadata_path.display(), backup_info.original_path.display());
            }
        }
    }
    
    /// Reconstruct likely original path for a downloaded backup
    fn reconstruct_likely_original_path(&self, game_name: &str, app_id: Option<u32>, save_type: &SaveType) -> std::path::PathBuf {
        use std::path::PathBuf;
        
        match save_type {
            SaveType::Steam => {
                // For Steam games, reconstruct the likely Steam userdata path
                if let Some(id) = app_id {
                    // Steam saves are typically in: Steam/userdata/{user_id}/{app_id}/remote/
                    // We'll use a generic user_id since we don't know which user
                    let steam_path = PathBuf::from(&self.config.steam_path)
                        .join("[Steam User]")
                        .join(id.to_string())
                        .join("remote");
                    return steam_path;
                } else {
                    // Fallback for Steam games without app ID
                    return PathBuf::from(format!("Steam Save Location - {}", game_name));
                }
            },
            SaveType::NonSteam => {
                // For non-Steam games, try common locations
                let clean_name = game_name.replace(' ', "").replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "");
                
                // Try Documents first (most common)
                if let Some(docs_dir) = dirs::document_dir() {
                    return docs_dir.join("My Games").join(&clean_name);
                }
                
                // Fallback to AppData
                if let Some(appdata_dir) = dirs::data_local_dir() {
                    return appdata_dir.join(&clean_name);
                }
                
                // Final fallback
                return PathBuf::from(format!("Documents/My Games/{}", clean_name));
            }
        }
    }
    
    fn full_sync_koofr(&mut self) {
        info!("Starting full Koofr sync");
        
        if !self.config.koofr_config.enabled {
            self.scan_status = ScanStatus::Error("Koofr sync is not enabled".to_string());
            return;
        }
        
        self.scan_status = ScanStatus::Scanning;
        
        // Initialize cloud folder first
        match self.initialize_cloud_folder() {
            Ok(()) => {
                info!("Cloud folder initialized successfully");
                self.scan_status = ScanStatus::Complete("Cloud folder ready. Starting sync...".to_string());
            },
            Err(e) => {
                warn!("Failed to initialize cloud folder: {}", e);
                // Continue anyway - might already exist
                self.scan_status = ScanStatus::Complete("Cloud folder may already exist. Continuing sync...".to_string());
            }
        }
        
        // First, try to list what's in the cloud
        std::thread::sleep(std::time::Duration::from_millis(300));
        self.download_backups_from_koofr();
        
        // Wait a moment, then upload local backups
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.upload_backups_to_koofr();
    }
}
