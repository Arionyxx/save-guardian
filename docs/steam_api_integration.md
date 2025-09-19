# Steam API Integration for Dynamic Game Name Fetching

## Overview

Save Guardian now includes dynamic game name fetching from online sources, eliminating the need for hardcoded game lists. This feature significantly improves the accuracy of game names displayed in the application.

## Features

### 1. **Dynamic Game Name Resolution**
- Queries Steam Store API and SteamSpy API for accurate game names
- Falls back to local Steam registry/config files if API calls fail
- Handles both popular and obscure Steam games automatically

### 2. **Persistent Caching**
- Game names are cached locally in `%APPDATA%/SaveGuardian/steam_game_cache.json`
- Cache persists across app restarts, reducing API calls
- Automatically saves new entries as they're fetched

### 3. **API Sources**
The system uses two free APIs in sequence:

1. **Steam Store API** (Primary)
   - URL: `https://store.steampowered.com/api/appdetails?appids={app_id}&filters=basic`
   - Most reliable for official Steam games
   - No API key required

2. **SteamSpy API** (Fallback)  
   - URL: `https://steamspy.com/api.php?request=appdetails&appid={app_id}`
   - Community-maintained database
   - Good for older or lesser-known games

## Code Changes

### SteamScanner Updates

The `SteamScanner` struct now includes:

```rust
pub struct SteamScanner {
    steam_userdata_path: PathBuf,
    app_cache: HashMap<u32, String>, // App ID -> Game Name
    cache_file_path: PathBuf,        // New: persistent cache location
}
```

### Key Methods Added

#### `get_game_name(app_id: u32) -> String`
- Now public for external use
- Checks cache first, then queries APIs if needed
- Returns formatted fallback for unknown games

#### `fetch_game_name_from_api(app_id: u32) -> Result<String, Error>`
- Handles API requests with 5-second timeouts
- Tries Steam Store API first, then SteamSpy
- Includes proper User-Agent headers

#### `refresh_game_names()`
- Updates all cached entries from online sources
- Includes rate limiting (100ms delay between requests)
- Useful for updating outdated cache entries

#### `clear_cache()`
- Clears in-memory cache and removes cache file
- Useful for troubleshooting or starting fresh

#### `get_cache_stats() -> (usize, String)`
- Returns cache size and file information
- Helpful for debugging and status displays

## Usage Examples

### Basic Usage
```rust
use save_guardian::steam::SteamScanner;
use std::path::PathBuf;

let mut scanner = SteamScanner::new(steam_path);
let game_name = scanner.get_game_name(730); // Counter-Strike 2
println!("Game: {}", game_name);
```

### Cache Management
```rust
// Check cache statistics
let (count, info) = scanner.get_cache_stats();
println!("Cached games: {}, {}", count, info);

// Refresh all cached names
scanner.refresh_game_names();

// Clear cache if needed
scanner.clear_cache();
```

## Configuration

No configuration is required - the system works out of the box with sensible defaults:

- **Cache Location**: `%APPDATA%/SaveGuardian/steam_game_cache.json`
- **API Timeout**: 5 seconds per request
- **Rate Limiting**: 100ms delay when bulk refreshing
- **Fallback Behavior**: Uses existing Steam registry/config methods

## Error Handling

The system gracefully handles various failure scenarios:

- **Network Issues**: Falls back to local Steam data
- **API Rate Limits**: Implements delays and timeouts
- **Invalid App IDs**: Returns descriptive fallback names
- **Cache Corruption**: Rebuilds cache from scratch

## Performance Considerations

- **First Run**: Slower as game names are fetched and cached
- **Subsequent Runs**: Fast lookups from local cache
- **Bandwidth**: Minimal - only fetches names as needed
- **Storage**: Cache file typically <50KB for hundreds of games

## Testing

Run the included example to test the functionality:

```bash
cargo run --example test_steam_api
```

This will demonstrate:
- Fetching names for popular Steam games
- Cache creation and persistence
- Error handling for non-existent games
- Performance with and without cache

## Future Enhancements

Potential improvements for future versions:

1. **Additional APIs**: Integration with SteamDB or other sources
2. **Background Updates**: Async cache refreshing
3. **User Overrides**: Allow manual name customization
4. **Batch Fetching**: Request multiple game names simultaneously
5. **Localization**: Support for different languages

## Migration

Existing Save Guardian installations will automatically benefit from this feature:

- No user action required
- Existing hardcoded names are gradually replaced
- Cache builds up naturally during normal usage
- No breaking changes to existing functionality

## Troubleshooting

### Common Issues

1. **"Unknown Game" Names**
   - Check internet connection
   - Verify the Steam App ID is correct
   - Try clearing the cache: `scanner.clear_cache()`

2. **Slow Performance**
   - First-time API calls are slower
   - Subsequent runs use cached data
   - Check network connectivity

3. **Cache Issues**
   - Cache file location: `%APPDATA%/SaveGuardian/steam_game_cache.json`
   - Safe to delete if corrupted
   - Will rebuild automatically

### Debug Logging

Enable debug logging to see API activity:

```bash
RUST_LOG=debug cargo run
```

This shows:
- API request attempts
- Cache hits/misses
- Error conditions
- Performance timing