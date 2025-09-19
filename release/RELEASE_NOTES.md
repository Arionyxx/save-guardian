# Save Guardian v1.0.0 Release Notes

## 🎉 First Release

Save Guardian is a powerful save game manager for Steam and non-Steam games, built with Rust and egui.

### ✨ Features

- **🔍 Automatic Save Detection**: Detects Steam and non-Steam game saves automatically
- **💾 Backup & Restore**: Create and manage backups with descriptions and timestamps
- **☁️ Cloud Sync**: Upload/download backups to Koofr WebDAV storage
- **🎮 Steam API Integration**: Fetches correct game names from Steam APIs
- **📂 Easy Management**: One-click access to backup folders
- **🔄 Save Synchronization**: Sync saves between Steam and non-Steam versions
- **🎨 Modern UI**: Clean interface with dark/light theme support

### 🐛 Known Issues

⚠️ **Game Name Display Issue**: Some games may show incorrect names in the main list.
- **Workaround**: Use the search box to find games, then click backup
- **Alternative**: Click the "↻ Refresh" button to update all names
- **Root Cause**: Cached names haven't been refreshed from Steam API yet

### 📋 System Requirements

- Windows 10 or later
- Steam installation (for Steam save detection)
- Internet connection (for game name fetching)

### 🚀 Installation

1. Download `save-guardian.exe` from this release
2. Place it in any folder of your choice
3. Run the executable - no installation required!

### 🎮 Supported Games

- All Steam games with Cloud Save support
- Non-Steam games in standard locations:
  - Documents/My Games
  - AppData/Roaming
  - AppData/Local
  - AppData/LocalLow (Unity games)
  - Custom locations (configurable)

### ⚙️ Optional Cloud Setup

1. Sign up for a free Koofr account
2. Go to Settings → Cloud Sync
3. Enable Koofr and enter your WebDAV credentials
4. Use the Cloud tab to upload/download backups

### 📊 Technical Details

- **Built with**: Rust 🦀 + egui GUI framework
- **Backup format**: ZIP archives with JSON metadata
- **Cloud protocol**: WebDAV (Koofr compatible)
- **Game detection**: Steam API integration + filesystem scanning

### 🔧 Usage Tips

1. **First Run**: Let the app scan your saves (may take a moment)
2. **Creating Backups**: Go to Game Saves tab, click 💾 Backup button
3. **Managing Backups**: Use the Backups tab to view and organize
4. **Cloud Sync**: Configure in Settings, then use Cloud tab
5. **Name Issues**: If game names look wrong, use the search box first

### 🤝 Contributing

This is an open-source project! Feel free to:
- Report bugs or issues
- Suggest new features
- Submit pull requests
- Help with documentation

### 📝 License

MIT License - see LICENSE file for details.

---

**Enjoy saving your game progress!** 🛡️