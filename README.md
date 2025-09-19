# 🛡️ Save Guardian

A sleek, modern save manager for Steam and non-Steam games built in Rust with a beautiful GUI.

## Features

✨ **Automatic Save Detection**
- Scans Steam userdata folders automatically
- Detects saves in common locations (Documents, AppData, etc.)
- Recognizes game names and Steam App IDs
- Supports custom save locations

🔐 **Backup & Restore**
- Create compressed backups of game saves
- Restore saves from any backup point  
- Automatic cleanup of old backups
- Backup with custom descriptions and metadata

🔄 **Save Synchronization**
- Sync saves between Steam and non-Steam versions
- Intelligent game matching by name and App ID
- Bidirectional sync based on modification times
- Automatic backup before sync operations

🎨 **Modern UI**
- Clean, intuitive interface built with egui
- Dark/Light/System theme support
- Search and filter capabilities
- Real-time scanning progress

## Screenshots

![Main Interface](docs/screenshot-main.png)
![Backup Management](docs/screenshot-backups.png)
![Sync Interface](docs/screenshot-sync.png)

## Installation

### Prerequisites

- Windows 10/11 (primary support)
- Steam installed (for Steam save detection)

### Download

1. Download the latest release from the [Releases page](https://github.com/username/save-guardian/releases)
2. Extract the ZIP file
3. Run `save-guardian.exe`

### Build from Source

```bash
# Clone the repository
git clone https://github.com/username/save-guardian.git
cd save-guardian

# Build the project
cargo build --release

# Run the application
cargo run --release
```

## Usage

### Initial Setup

1. **Launch Save Guardian**
2. **Configure Paths** in Settings:
   - Steam userdata path (usually auto-detected)
   - Backup directory (defaults to Documents/SaveGuardianBackups)
3. **Click Refresh** to scan for saves

### Managing Game Saves

1. **Game Saves Tab** shows all detected saves
2. **Search** for specific games or filter by type
3. **Backup** individual saves with custom descriptions
4. **Open** save directories in Windows Explorer

### Backup Management

1. **Backups Tab** shows all created backups
2. **Restore** saves from any backup point
3. **Delete** old or unnecessary backups
4. **Cleanup Old** automatically removes backups older than retention period

### Save Synchronization

1. **Sync Tab** shows potential sync pairs
2. **Find Pairs** to automatically detect matching games
3. **Sync** saves between Steam and non-Steam versions
4. Choose sync direction or use automatic bidirectional sync

## Supported Save Locations

### Steam Saves
- `C:\Program Files (x86)\Steam\userdata\{USER_ID}\{APP_ID}\remote\`
- Additional Steam Cloud locations

### Non-Steam Saves
- `Documents\My Games\`
- `Documents\{Publisher}\{Game}\`
- `Documents\Rockstar Games\`
- `AppData\Roaming\{Game}\`
- `AppData\Local\{Game}\`
- `AppData\LocalLow\{Company}\{Game}\` (Unity games)
- `C:\Users\Public\Documents\`
- `AppData\Roaming\Goldberg SteamEmu Saves\`
- Game installation directories
- Custom locations (user-defined)

## Configuration

Settings are automatically saved and include:

```toml
# Steam installation path
steam_path = "C:\\Program Files (x86)\\Steam\\userdata"

# Backup storage location  
backup_path = "C:\\Users\\{User}\\Documents\\SaveGuardianBackups"

# Backup retention period (days)
backup_retention_days = 30

# Automatic backup before operations
auto_backup = true

# UI theme
theme = "Dark" # "Light", "Dark", or "System"

# Window size and position
window_size = [1200.0, 800.0]
```

## Architecture

Save Guardian is built with a modular architecture:

- **`types.rs`** - Core data structures and error types
- **`steam.rs`** - Steam save detection and scanning
- **`non_steam.rs`** - Non-Steam save location scanning
- **`backup.rs`** - Backup creation, restoration, and management
- **`sync.rs`** - Save synchronization between Steam/non-Steam
- **`gui.rs`** - Modern UI implementation with egui
- **`config.rs`** - Configuration management

## Supported Games

Save Guardian works with thousands of games including:

- **Steam Games**: All games with Steam Cloud saves
- **Non-Steam Games**: Most PC games that store saves in standard locations
- **Cracked Games**: Games using common emulators like Goldberg
- **Unity Games**: Games storing data in LocalLow
- **Rockstar Games**: GTA series, Red Dead Redemption, etc.
- **EA Games**: Games storing saves in Documents
- **Ubisoft Games**: Games with standard save patterns

## ⚠️ Known Issues

### Game Name Display Issue

**Issue**: Some games may display incorrect names in the main game list (e.g., generic names like "Satisfactory" instead of the actual game name).

**Workaround**: 
1. **Use the search box** - Type the game name in the search box, then click backup. This ensures correct names are used.
2. **Use the Refresh button** - Click the "↻ Refresh" button to force update all game names.
3. **Automatic fix** - The app will automatically attempt to fix incorrect names on startup.

**Why this happens**: This occurs when cached game names haven't been updated from the Steam API yet. The search function triggers a refresh that fixes the names.

## Troubleshooting

### Steam Saves Not Detected
- Verify Steam is installed and has been run at least once
- Check Steam userdata path in Settings
- Ensure you have saves for the games (play them first)

### Non-Steam Saves Missing
- Check if games store saves in non-standard locations
- Add custom save locations in Settings
- Some games may use registry or other storage methods

### Backup/Restore Issues
- Ensure backup directory has sufficient space
- Check file permissions on save directories
- Some games may lock save files while running

### Sync Problems
- Ensure both save locations exist and are accessible
- Games must be closed during sync operations
- Check that save formats are compatible between versions

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/username/save-guardian.git
cd save-guardian
cargo run
```

### Adding New Save Locations

To add support for new save locations:

1. Update `non_steam.rs` with new location patterns
2. Add game-specific detection logic
3. Test with multiple games
4. Update documentation

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [egui](https://github.com/emilk/egui) for the modern GUI
- Uses [walkdir](https://github.com/BurntSushi/walkdir) for efficient directory traversal
- Inspired by game save managers like GameSave Manager

## Roadmap

- [ ] Support for Linux and macOS
- [ ] Cloud backup integration (Google Drive, Dropbox, etc.)
- [ ] Automatic save monitoring and backup
- [ ] Game launcher integration
- [ ] Save file diff and merge capabilities
- [ ] Portable mode for USB drives
- [ ] Command line interface
- [ ] Save game screenshots and metadata

## Support

- 🐛 **Bug Reports**: [Issues page](https://github.com/username/save-guardian/issues)
- 💡 **Feature Requests**: [Discussions page](https://github.com/username/save-guardian/discussions)  
- 📧 **Contact**: [your.email@example.com](mailto:your.email@example.com)

---

**Save Guardian** - Keep your game progress safe! 🛡️