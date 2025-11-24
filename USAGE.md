# SiegeSaver GUI Layout

## Application Window

The SiegeSaver application features a clean, native desktop interface with the following layout:

```
┌─────────────────────────────────────────────────────────────────┐
│  SiegeSaver - Replay File Backup Utility                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Source Folder (to watch for .rec files):                 │  │
│  │ ┌──────────────────────────────────┐ ┌────────┐          │  │
│  │ │ /path/to/source/folder           │ │Browse  │          │  │
│  │ └──────────────────────────────────┘ └────────┘          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Destination Folder (where backups will be saved):        │  │
│  │ ┌──────────────────────────────────┐ ┌────────┐          │  │
│  │ │ /path/to/destination/folder      │ │Browse  │          │  │
│  │ └──────────────────────────────────┘ └────────┘          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌────────────────┐  ● Watching                               │
│  │ Start Watching │  (Green indicator when active)            │
│  └────────────────┘  ○ Not Watching (Gray when inactive)      │
│                                                                 │
│  ☐ Start on system boot                                        │
│                                                                 │
│  ────────────────────────────────────────────────────────────  │
│                                                                 │
│  Status Messages:                                              │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ [12:34:56] Started watching: /path/to/source/folder      │  │
│  │ [12:35:01] Backed up: game_replay_001.rec               │  │
│  │ [12:35:15] Backed up: game_replay_002.rec               │  │
│  │ [12:36:22] Backed up: game_replay_003.rec               │  │
│  │                                                           │  │
│  │                                                           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Features

### 1. Folder Selection
- **Source Folder**: Text field with Browse button to select the folder to watch
- **Destination Folder**: Text field with Browse button to select backup location
- Both fields support manual text entry or folder selection via native file dialog

### 2. Watch Control
- **Start Watching** button: Initiates file monitoring
- **Stop Watching** button: Stops file monitoring (replaces Start when active)
- Status indicator (● or ○) shows current monitoring state
  - Green ● when watching
  - Gray ○ when not watching

### 3. Start on System Boot
- **Checkbox**: "Start on system boot" option
- When enabled, registers the application to start automatically when the system boots
- Works cross-platform (Windows, macOS, Linux)
- Setting is persisted and can be toggled at any time

### 4. Configuration Persistence
- Application settings are automatically saved to disk
- Settings saved include:
  - Source folder path
  - Destination folder path
  - Start on boot preference
- Configuration is restored when the application is relaunched
- Config stored at `~/.config/siegesaver/config.json` (or platform equivalent)

### 5. Status Log
- Scrollable area showing timestamped status messages
- Auto-scrolls to show latest messages
- Displays:
  - When watching starts/stops
  - Files successfully backed up
  - Any errors encountered
- Keeps last 100 messages to prevent memory bloat

## User Workflow

1. Launch SiegeSaver
2. Click "Browse" next to Source Folder → Select game replay directory
3. Click "Browse" next to Destination Folder → Select backup location
4. (Optional) Check "Start on system boot" to automatically launch SiegeSaver on system startup
5. Click "Start Watching"
6. Application now monitors source folder and automatically copies any new .rec files
7. Status log shows real-time backup activity
8. Click "Stop Watching" to pause monitoring

Settings are automatically saved and will be restored when you launch the app again.

## Technical Implementation

- **GUI Framework**: eframe/egui (native, not web-based)
- **File Watching**: notify crate with 500ms debouncing
- **Threading**: Background thread handles file copy operations
- **File Dialogs**: rfd (Rusty File Dialogs) for native OS dialogs
- **Auto-Launch**: auto-launch crate for cross-platform startup integration
- **Configuration**: serde/serde_json for JSON serialization
- **Platform Directories**: dirs crate for platform-specific config paths
