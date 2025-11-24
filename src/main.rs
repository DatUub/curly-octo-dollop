// Copyright (C) 2025 DatUub | Licensed under GPL-3.0 (see LICENSE file)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use eframe::egui;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, MouseButton, TrayIconBuilder, TrayIconEvent,
};

// --- CONFIGURATION ---
#[derive(Serialize, Deserialize, Default, Clone)]
struct AppConfig {
    start_on_boot: bool,
    source_folder: String,
    destination_folder: String,
}

impl AppConfig {
    fn load() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("siegesaver").join("config.json");
            if let Ok(contents) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    fn save(&self) {
        if let Some(config_dir) = dirs::config_dir() {
            let config_dir = config_dir.join("siegesaver");
            if fs::create_dir_all(&config_dir).is_ok() {
                let config_path = config_dir.join("config.json");
                if let Ok(json) = serde_json::to_string_pretty(self) {
                    let _ = fs::write(&config_path, json);
                }
            }
        }
    }
}

// --- SYSTEM UTILS ---
fn get_auto_launch() -> Result<AutoLaunch, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    AutoLaunchBuilder::new()
        .set_app_name("SiegeSaver")
        .set_app_path(&exe_path.to_string_lossy())
        .build()
        .map_err(|e| e.to_string())
}

// --- ICON LOADING ---
fn load_icon_for_window() -> std::sync::Arc<egui::IconData> {
    match load_icon_data_from_file() {
        Ok(icon_data) => std::sync::Arc::new(icon_data),
        Err(_) => std::sync::Arc::new(create_fallback_icon_data()),
    }
}

fn load_icon_bytes_from_file() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let icon_path = std::env::current_exe()?.parent().unwrap().join("icon.ico");
    if icon_path.exists() {
        Ok(std::fs::read(&icon_path)?)
    } else {
        Ok(std::fs::read("icon.ico")?)
    }
}

fn load_icon_data_from_file() -> Result<egui::IconData, Box<dyn std::error::Error>> {
    let icon_data = load_icon_bytes_from_file()?;
    let img = image::load_from_memory(&icon_data)?;
    let rgba_image = img.to_rgba8();
    let (width, height) = rgba_image.dimensions();
    Ok(egui::IconData {
        rgba: rgba_image.into_raw(),
        width,
        height,
    })
}

fn create_fallback_icon_data() -> egui::IconData {
    let mut rgba = Vec::with_capacity(16 * 16 * 4);
    for _ in 0..(16 * 16) {
        rgba.extend_from_slice(&[255, 0, 0, 255]); // Red block fallback
    }
    egui::IconData {
        rgba,
        width: 16,
        height: 16,
    }
}

fn load_icon() -> Icon {
    match load_icon_from_file() {
        Ok(icon) => icon,
        Err(_) => create_fallback_icon(),
    }
}

fn load_icon_from_file() -> Result<Icon, Box<dyn std::error::Error>> {
    let icon_data = load_icon_bytes_from_file()?;
    let img = image::load_from_memory(&icon_data)?;
    let rgba = img.to_rgba8();
    Icon::from_rgba(rgba.to_vec(), rgba.width(), rgba.height()).map_err(|e| e.into())
}

fn create_fallback_icon() -> Icon {
    let rgba = vec![255u8; 16 * 16 * 4];
    Icon::from_rgba(rgba, 16, 16).expect("Failed to create fallback icon")
}

// --- MAIN ENTRY POINT ---
fn main() -> Result<(), eframe::Error> {
    let menu = Menu::new();
    let quit_item = MenuItem::new("Quit SiegeSaver", true, None);
    menu.append(&quit_item).unwrap();

    // Load icon once
    let tray_icon_data = load_icon();
    
    // Create tray icon but DON'T drop it
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("SiegeSaver Running")
        .with_icon(tray_icon_data)
        .build()
        .unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 450.0])
            .with_icon(load_icon_for_window()),
        ..Default::default()
    };

    eframe::run_native(
        "SiegeSaver",
        options,
        Box::new(move |cc| {
            // Pass the tray_icon into the app struct so it survives!
            Ok(Box::new(SiegeSaverApp::new(cc, quit_item.id().clone(), tray_icon)))
        }),
    )
}

// --- APP STATE ---
struct SiegeSaverApp {
    source_folder: String,
    destination_folder: String,
    watcher: Option<Arc<Mutex<Debouncer<notify::RecommendedWatcher, FileIdMap>>>>,
    status_messages: VecDeque<String>,
    is_watching: bool,
    status_receiver: Option<Receiver<String>>,
    start_on_boot: bool,
    quit_item_id: tray_icon::menu::MenuId,
    should_exit: bool,
    _tray_icon: tray_icon::TrayIcon, // Kept alive here
}

impl SiegeSaverApp {
    fn new(_cc: &eframe::CreationContext<'_>, quit_item_id: tray_icon::menu::MenuId, tray_icon: tray_icon::TrayIcon) -> Self {
        let config = AppConfig::load();
        Self {
            source_folder: config.source_folder,
            destination_folder: config.destination_folder,
            watcher: None,
            status_messages: VecDeque::new(),
            is_watching: false,
            status_receiver: None,
            start_on_boot: config.start_on_boot,
            quit_item_id,
            should_exit: false,
            _tray_icon: tray_icon,
        }
    }

    fn add_status(&mut self, msg: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        self.status_messages.push_front(format!("[{}] {}", timestamp, msg));
        if self.status_messages.len() > 50 {
            self.status_messages.pop_back();
        }
    }

    fn start_watching(&mut self) {
        let source = PathBuf::from(&self.source_folder);
        let dest = PathBuf::from(&self.destination_folder);

        // Validate paths
        if !source.exists() {
            self.add_status(format!("Source not found: {}", self.source_folder));
            return;
        }
        // Try to resolve absolute paths to avoid prefix stripping issues
        let canonical_source = fs::canonicalize(&source).unwrap_or(source.clone());
        
        if !dest.exists() {
            let _ = fs::create_dir_all(&dest);
        }

        let (tx, rx) = channel();
        let (status_tx, status_rx) = channel();
        let status_tx_clone = status_tx.clone();

        // Create debouncer with short timeout to catch rapid file updates
        let mut debouncer = new_debouncer(
            Duration::from_millis(500), 
            None,
            move |res: DebounceEventResult| {
                match res {
                    Ok(events) => {
                        for event in events {
                            let _ = tx.send(event.event);
                        }
                    },
                    Err(e) => { let _ = status_tx_clone.send(format!("Watch error: {:?}", e)); }
                }
            }
        ).unwrap();

        if let Err(e) = debouncer.watcher().watch(&canonical_source, RecursiveMode::Recursive) {
            self.add_status(format!("Failed to watch: {}", e));
            return;
        }

        self.watcher = Some(Arc::new(Mutex::new(debouncer)));
        self.is_watching = true;
        self.status_receiver = Some(status_rx);
        self.add_status("Monitoring started.".to_string());

        // Spawn background thread
        let source_clone = canonical_source.clone();
        let dest_clone = dest.clone();
        std::thread::spawn(move || {
            handle_watch_events(rx, source_clone, dest_clone, status_tx);
        });
    }

    fn stop_watching(&mut self) {
        self.watcher = None;
        self.is_watching = false;
        self.add_status("Monitoring stopped.".to_string());
    }
}

// --- FILE LOGIC ---
fn handle_watch_events(
    rx: Receiver<Event>,
    source_root: PathBuf,
    dest_root: PathBuf,
    status_tx: Sender<String>,
) {
    while let Ok(event) = rx.recv() {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in event.paths {
                    // logic: We want to mirror everything from source_root to dest_root
                    // If it's a file, copy it. If it's a dir, ensure it exists.
                    
                    if path.is_file() {
                        // Check extension
                        if let Some(ext) = path.extension() {
                            if ext == "rec" || ext == "match" { // Handle both likely extensions
                                process_file_copy(&path, &source_root, &dest_root, &status_tx);
                            }
                        }
                    } else if path.is_dir() {
                        // If a new directory is created (e.g. Match-2025...), scan it
                        // recursively immediately to catch any files that might already be inside
                        if let Ok(entries) = fs::read_dir(&path) {
                            for entry in entries.flatten() {
                                if entry.path().is_file() {
                                    process_file_copy(&entry.path(), &source_root, &dest_root, &status_tx);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn process_file_copy(path: &Path, source_root: &Path, dest_root: &Path, status_tx: &Sender<String>) {
    // Calculate relative path safely
    // We rely on the fact that we canonicalized source_root earlier.
    // We try to canonicalize the file path too for matching.
    let canon_path = fs::canonicalize(path).unwrap_or(path.to_path_buf());
    
    if let Ok(rel_path) = canon_path.strip_prefix(source_root) {
        let dest_path = dest_root.join(rel_path);

        // Ensure parent dir exists
        if let Some(parent) = dest_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Perform Copy
        match fs::copy(&canon_path, &dest_path) {
            Ok(_) => {
                let _ = status_tx.send(format!("Backed up: {}", rel_path.display()));
            }
            Err(e) => {
                let _ = status_tx.send(format!("Copy failed for {:?}: {}", rel_path, e));
            }
        }
    }
}

// --- GUI IMPLEMENTATION ---
impl eframe::App for SiegeSaverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Handle Status Updates from Thread
        if let Some(rx) = &self.status_receiver {
            while let Ok(msg) = rx.try_recv() {
                self.add_status(msg);
            }
        }

        // 2. Handle Tray Events
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        // 3. Handle Menu Events (Quit)
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.quit_item_id {
                self.should_exit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // 4. Handle Window Close Request (The "X" Button)
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.should_exit {
                // Actually close
            } else {
                // Just hide
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        // 5. Draw UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SiegeSaver Backup Tool");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Source:");
                if ui.button("Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.source_folder = path.display().to_string();
                        let cfg = AppConfig { start_on_boot: self.start_on_boot, source_folder: self.source_folder.clone(), destination_folder: self.destination_folder.clone() };
                        cfg.save();
                    }
                }
            });
            ui.text_edit_singleline(&mut self.source_folder);

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Backup To:");
                if ui.button("Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.destination_folder = path.display().to_string();
                        let cfg = AppConfig { start_on_boot: self.start_on_boot, source_folder: self.source_folder.clone(), destination_folder: self.destination_folder.clone() };
                        cfg.save();
                    }
                }
            });
            ui.text_edit_singleline(&mut self.destination_folder);

            ui.add_space(10.0);

            if self.is_watching {
                if ui.button("STOP WATCHING").clicked() {
                    self.stop_watching();
                }
                ui.colored_label(egui::Color32::GREEN, "System Active - Minimizes to Tray");
            } else {
                if ui.button("START WATCHING").clicked() {
                    self.start_watching();
                }
            }

            ui.separator();
            ui.label("Activity Log:");
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for msg in &self.status_messages {
                    ui.label(msg);
                }
            });
        });
    }
}
