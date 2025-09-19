use save_guardian::steam::SteamScanner;
use std::path::PathBuf;

/// Example demonstrating the Steam API game name fetching functionality
/// 
/// This example shows how the SteamScanner now:
/// 1. Queries Steam Store API and SteamSpy API for game names
/// 2. Caches results locally to reduce API calls
/// 3. Persists cache across app restarts
fn main() {
    // Initialize logging
    env_logger::init();
    
    println!("=== Steam API Game Name Fetching Test ===\n");
    
    // Create a SteamScanner (steam_path doesn't matter for this test)
    let mut scanner = SteamScanner::new(PathBuf::from("."));
    
    // Test some popular Steam app IDs
    let test_apps = vec![
        (730, "Counter-Strike 2"),           // CS2
        (440, "Team Fortress 2"),            // TF2  
        (570, "Dota 2"),                     // Dota 2
        (1172470, "Apex Legends"),           // Apex
        (271590, "Grand Theft Auto V"),      // GTA V
        (292030, "The Witcher 3"),           // Witcher 3
        (999999999, "This shouldn't exist"), // Non-existent game
    ];
    
    println!("Testing game name fetching for popular Steam games:");
    println!("(Note: This requires internet connection)\n");
    
    for (app_id, expected_name) in test_apps {
        print!("App ID {}: ", app_id);
        
        // This will now try online APIs first, then fall back to local data
        let fetched_name = scanner.get_game_name(app_id);
        
        if fetched_name.starts_with("Unknown Game") {
            println!("❌ Could not fetch name (got '{}')", fetched_name);
        } else {
            println!("✅ '{}' (expected: '{}')", fetched_name, expected_name);
        }
    }
    
    // Show cache statistics
    let (cache_size, cache_info) = scanner.get_cache_stats();
    println!("\n=== Cache Statistics ===");
    println!("Games cached: {}", cache_size);
    println!("{}", cache_info);
    
    // Example of refreshing cache (uncomment to test)
    // println!("\n=== Refreshing Cache ===");
    // scanner.refresh_game_names();
    
    println!("\n=== Test Complete ===");
    println!("The cache file has been created and will persist game names for future runs.");
    println!("This reduces the need to query APIs repeatedly for the same games.");
}