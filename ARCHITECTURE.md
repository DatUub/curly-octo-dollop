# SiegeSaver Architecture

## Overview

SiegeSaver is a native desktop application built with Rust that monitors a source folder for `.rec` files and automatically backs them up to a destination folder. The application ensures backups persist even when source files are deleted.

## Technology Stack

- **Language**: Rust 2021 Edition
- **GUI Framework**: eframe/egui (native, lightweight)
- **File Watching**: notify + notify-debouncer-full
- **File Dialogs**: rfd (Rusty File Dialogs)
- **Time Handling**: chrono
- **Configuration**: serde + serde_json (JSON serialization)
- **Auto-Launch**: auto-launch (cross-platform startup integration)
- **Directories**: dirs (platform-specific paths)

## Architecture

### Main Components

1. **AppConfig** (Configuration Management)
   - Stores application settings (source/dest folders, start_on_boot)
   - JSON serialization via serde
   - Automatic save/load from platform config directory
   - Persistent across application restarts

2. **SiegeSaverApp** (Main GUI Application)
   - Manages application state
   - Handles user interactions
   - Displays status messages
   - Receives updates from background thread via channel
   - Manages auto-launch registration

3. **File Watcher** (notify-debouncer-full)
   - Monitors source folder for file system events
   - 500ms debouncing to avoid duplicate events
   - Recursive monitoring of source directory

4. **Event Handler Thread** (Background)
   - Processes file system events
   - Copies `.rec` files to destination
   - Sends status updates to GUI via channel

5. **Auto-Launch Integration**
   - Cross-platform startup registration
   - Uses OS-specific mechanisms (Registry on Windows, Login Items on macOS, autostart on Linux)
   - Managed through auto-launch crate

### Data Flow

```
User Selects Folders → GUI
                        ↓
                   Start Watching
                        ↓
                Create File Watcher → Background Thread
                        ↓                     ↓
                Monitor Source ←──────────────┘
                        ↓
                 .rec File Created
                        ↓
              Copy to Destination
                        ↓
            Send Status via Channel
                        ↓
                   GUI Updates
```

### Key Design Decisions

#### 1. Configuration Persistence
- Settings stored in platform-specific config directory
- JSON format for human readability and easy debugging
- Automatic save on any setting change
- Load on application startup
- Config location: `~/.config/siegesaver/config.json` (or platform equivalent)

#### 2. VecDeque for Status Messages
- Uses `VecDeque<String>` instead of `Vec<String>` for O(1) front removal
- Maintains last 100 messages for performance
- Automatically scrolls to show latest messages

#### 3. Channel-Based Communication
- Background thread sends status updates to GUI via `mpsc::channel`
- Avoids console output (not visible in GUI applications)
- All messages displayed in scrollable status log

#### 4. File Extension Filtering
- Only processes files with `.rec` extension
- Case-sensitive matching (as per typical file system behavior)

#### 5. Persistent Backups
- Copies files to destination, doesn't move them
- Destination files independent of source files
- Source file deletion doesn't affect backups

#### 6. Cross-Platform Auto-Launch
- Uses auto-launch crate for OS-specific startup integration
- Windows: Registry entry in HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Run
- macOS: Launch Agent or AppleScript login items
- Linux: .desktop file in ~/.config/autostart/

#### 7. Error Handling
- All errors reported to GUI via status channel
- Graceful degradation on errors
- User-friendly error messages

### Thread Safety

- File watcher wrapped in `Arc<Mutex<>>` for shared ownership
- Channel-based communication is thread-safe by design
- No shared mutable state between threads

### Performance Considerations

- Debounced file watching (500ms) reduces CPU usage
- VecDeque for efficient message queue management
- Background thread prevents GUI blocking
- Direct file copy (no buffering layer needed)

## Building and Testing

### Development Build
```bash
cargo build
```

### Release Build
```bash
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Code Quality
```bash
cargo clippy
cargo fmt
```

## Future Enhancements (Potential)

1. Configurable file patterns (not just `.rec`)
2. Backup history/versioning
3. Compression options
4. Configuration persistence
5. Multiple source/destination pairs
6. Backup statistics (files backed up, total size, etc.)
7. File filtering by size or date

## Security Considerations

- File paths validated before operations
- No arbitrary code execution
- Standard Rust memory safety guarantees
- Native GUI (no web vulnerabilities)
- Minimal dependencies (reduced attack surface)
