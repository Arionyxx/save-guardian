mod types;
mod steam;
mod non_steam;
mod backup;
mod sync;
mod gui;
mod config;

use eframe::egui;
use gui::SaveGuardianApp;

fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    env_logger::init();
    
    // Set up eframe options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    // Run the application
    eframe::run_native(
        "Save Guardian",
        options,
        Box::new(|cc| Box::new(SaveGuardianApp::new(cc))),
    )
}
