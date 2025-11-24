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

### 3. Status Log
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
4. Click "Start Watching"
5. Application now monitors source folder and automatically copies any new .rec files
6. Status log shows real-time backup activity
7. Click "Stop Watching" to pause monitoring

## Technical Implementation

- **GUI Framework**: eframe/egui (native, not web-based)
- **File Watching**: notify crate with 500ms debouncing
- **Threading**: Background thread handles file copy operations
- **File Dialogs**: rfd (Rusty File Dialogs) for native OS dialogs
