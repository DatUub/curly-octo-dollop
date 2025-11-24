# SiegeSaver Architecture

## Overview

SiegeSaver is a native desktop application built with Rust that monitors a source folder for `.rec` files and automatically backs them up to a destination folder. The application ensures backups persist even when source files are deleted.

## Technology Stack

- **Language**: Rust 2021 Edition
- **GUI Framework**: eframe/egui (native, lightweight)
- **File Watching**: notify + notify-debouncer-full
- **File Dialogs**: rfd (Rusty File Dialogs)
- **Time Handling**: chrono

## Architecture

### Main Components

1. **SiegeSaverApp** (Main GUI Application)
   - Manages application state
   - Handles user interactions
   - Displays status messages
   - Receives updates from background thread via channel

2. **File Watcher** (notify-debouncer-full)
   - Monitors source folder for file system events
   - 500ms debouncing to avoid duplicate events
   - Recursive monitoring of source directory

3. **Event Handler Thread** (Background)
   - Processes file system events
   - Copies `.rec` files to destination
   - Sends status updates to GUI via channel

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

#### 1. VecDeque for Status Messages
- Uses `VecDeque<String>` instead of `Vec<String>` for O(1) front removal
- Maintains last 100 messages for performance
- Automatically scrolls to show latest messages

#### 2. Channel-Based Communication
- Background thread sends status updates to GUI via `mpsc::channel`
- Avoids console output (not visible in GUI applications)
- All messages displayed in scrollable status log

#### 3. File Extension Filtering
- Only processes files with `.rec` extension
- Case-sensitive matching (as per typical file system behavior)

#### 4. Persistent Backups
- Copies files to destination, doesn't move them
- Destination files independent of source files
- Source file deletion doesn't affect backups

#### 5. Error Handling
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
