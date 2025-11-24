# SiegeSaver - Replay File Backup Utility

A high-performance, lightweight desktop utility built with Rust and egui for automatically backing up match replay folders.

## Features

- **Real-time Folder Monitoring**: Watches a source folder for new match folders (e.g., `Match-2025-11-23-001`) using the `notify` crate with debounced file system events
- **Automatic Backup**: Instantly copies new match folders and all their contents to a destination folder
- **Folder Structure Preservation**: Maintains the original folder structure, keeping each match's files organized in their respective folders
- **Persistent Backups**: Destination folders persist even when source folders are deleted (e.g., due to game cleanup)
- **Start on System Boot**: Optional setting to automatically start SiegeSaver when the system boots
- **Persistent Configuration**: Settings are automatically saved and restored between sessions
- **Native GUI**: Built with eframe/egui for a responsive, lightweight desktop interface
- **Cross-platform**: Works on Windows, macOS, and Linux

## Tech Stack

- **Language**: Rust (2021 Edition)
- **GUI Framework**: eframe (egui) - Native GUI, not web-based
- **File Watching**: notify crate with debounced events
- **File Dialogs**: rfd (Rusty File Dialogs)

## Building

### Prerequisites

- Rust 1.70 or later (2021 edition)
- Cargo

### Build Instructions

```bash
# Clone the repository
git clone https://github.com/DatUub/curly-octo-dollop.git
cd curly-octo-dollop

# Build in release mode for optimal performance
cargo build --release

# Run the application
cargo run --release
```

## Usage

1. **Launch SiegeSaver**
2. **Select Source Folder**: Click "Browse" next to the Source Folder field and select the folder where match folders are created (e.g., the game's replay folder)
3. **Select Destination Folder**: Click "Browse" next to the Destination Folder field and select where you want backups saved
4. **Start Watching**: Click the "Start Watching" button to begin monitoring
5. **Enable Start on Boot** (Optional): Check the "Start on system boot" checkbox to automatically launch SiegeSaver when your system starts
6. **Status Updates**: View real-time status messages in the log area at the bottom

The application will automatically:
- Monitor the source folder for any new match folders (e.g., `Match-2025-11-23-001`)
- Copy entire folders and all their contents (including `.rec` files) to the destination folder immediately
- Preserve the folder structure (each match folder remains separate)
- Keep destination folders even if the source folders are deleted
- Save your folder selections and preferences for the next time you launch the app

## Development

### Running in Development Mode

```bash
cargo run
```

### Running Lints

```bash
cargo clippy
```

### Code Format

```bash
cargo fmt
```

## License

See LICENSE file for details.