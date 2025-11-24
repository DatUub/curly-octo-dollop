use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use eframe::egui;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Serialize, Deserialize, Default)]
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

fn get_auto_launch() -> Result<AutoLaunch, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    
    AutoLaunchBuilder::new()
        .set_app_name("SiegeSaver")
        .set_app_path(&exe_path.to_string_lossy())
        .build()
        .map_err(|e| e.to_string())
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "SiegeSaver - Replay File Backup Utility",
        options,
        Box::new(|_cc| Ok(Box::new(SiegeSaverApp::new()))),
    )
}

struct SiegeSaverApp {
    source_folder: String,
    destination_folder: String,
    watcher: Option<Arc<Mutex<Debouncer<notify::RecommendedWatcher, FileIdMap>>>>,
    status_messages: VecDeque<String>,
    is_watching: bool,
    status_receiver: Option<Receiver<String>>,
    start_on_boot: bool,
}

impl SiegeSaverApp {
    fn new() -> Self {
        let config = AppConfig::load();
        Self {
            source_folder: config.source_folder,
            destination_folder: config.destination_folder,
            watcher: None,
            status_messages: VecDeque::new(),
            is_watching: false,
            status_receiver: None,
            start_on_boot: config.start_on_boot,
        }
    }

    fn save_config(&self) {
        let config = AppConfig {
            start_on_boot: self.start_on_boot,
            source_folder: self.source_folder.clone(),
            destination_folder: self.destination_folder.clone(),
        };
        config.save();
    }

    fn add_status(&mut self, message: String) {
        self.status_messages.push_back(format!(
            "[{}] {}",
            chrono::Local::now().format("%H:%M:%S"),
            message
        ));
        // Keep only the last 100 messages
        if self.status_messages.len() > 100 {
            self.status_messages.pop_front();
        }
    }

    fn start_watching(&mut self) {
        if self.source_folder.is_empty() || self.destination_folder.is_empty() {
            self.add_status("Error: Please select both source and destination folders".to_string());
            return;
        }

        let source_path = PathBuf::from(&self.source_folder);
        let destination_path = PathBuf::from(&self.destination_folder);

        if !source_path.exists() {
            self.add_status(format!(
                "Error: Source folder does not exist: {}",
                self.source_folder
            ));
            return;
        }

        if !destination_path.exists() {
            if let Err(e) = fs::create_dir_all(&destination_path) {
                self.add_status(format!("Error creating destination folder: {}", e));
                return;
            }
            self.add_status(format!(
                "Created destination folder: {}",
                self.destination_folder
            ));
        }

        let dest_clone = destination_path.clone();
        let (tx, rx) = channel();
        let (status_tx, status_rx) = channel();
        let status_tx_clone = status_tx.clone();

        let mut debouncer = match new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| match result {
                Ok(events) => {
                    for event in events {
                        if let Err(e) = tx.send(event.event) {
                            let _ = status_tx_clone.send(format!("Error sending event: {}", e));
                        }
                    }
                }
                Err(errors) => {
                    for error in errors {
                        let _ = status_tx_clone.send(format!("File watch error: {:?}", error));
                    }
                }
            },
        ) {
            Ok(d) => d,
            Err(e) => {
                self.add_status(format!("Error creating file watcher: {}", e));
                return;
            }
        };

        if let Err(e) = debouncer
            .watcher()
            .watch(&source_path, RecursiveMode::Recursive)
        {
            self.add_status(format!("Error watching folder: {}", e));
            return;
        }

        self.watcher = Some(Arc::new(Mutex::new(debouncer)));
        self.is_watching = true;
        self.status_receiver = Some(status_rx);
        self.add_status(format!("Started watching: {}", self.source_folder));

        // Spawn a thread to handle file events
        let dest_for_thread = dest_clone;
        std::thread::spawn(move || {
            handle_file_events(rx, dest_for_thread, status_tx);
        });
    }

    fn stop_watching(&mut self) {
        self.watcher = None;
        self.is_watching = false;
        self.add_status("Stopped watching".to_string());
    }

    fn set_start_on_boot(&mut self, enabled: bool) {
        match get_auto_launch() {
            Ok(auto_launch) => {
                let result = if enabled {
                    auto_launch.enable()
                } else {
                    auto_launch.disable()
                };

                match result {
                    Ok(_) => {
                        self.start_on_boot = enabled;
                        self.save_config();
                        let status = if enabled { "enabled" } else { "disabled" };
                        self.add_status(format!("Start on system boot {}", status));
                    }
                    Err(e) => {
                        self.add_status(format!("Error setting start on boot: {}", e));
                    }
                }
            }
            Err(e) => {
                self.add_status(format!("Error accessing auto-launch: {}", e));
            }
        }
    }
}

fn handle_file_events(rx: Receiver<Event>, destination_folder: PathBuf, status_tx: Sender<String>) {
    while let Ok(event) = rx.recv() {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in event.paths {
                    if let Some(extension) = path.extension() {
                        if extension == "rec" {
                            if let Some(filename) = path.file_name() {
                                let dest_path = destination_folder.join(filename);
                                match fs::copy(&path, &dest_path) {
                                    Ok(_) => {
                                        let msg =
                                            format!("Backed up: {}", filename.to_string_lossy());
                                        let _ = status_tx.send(msg);
                                    }
                                    Err(e) => {
                                        let msg = format!(
                                            "Error copying {}: {}",
                                            filename.to_string_lossy(),
                                            e
                                        );
                                        let _ = status_tx.send(msg);
                                    }
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

impl eframe::App for SiegeSaverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for status messages from the background thread
        let mut messages = Vec::new();
        if let Some(receiver) = &self.status_receiver {
            while let Ok(msg) = receiver.try_recv() {
                messages.push(msg);
            }
        }
        for msg in messages {
            self.add_status(msg);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SiegeSaver - Replay File Backup Utility");
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Source Folder (to watch for .rec files):");
                ui.horizontal(|ui| {
                    if ui.text_edit_singleline(&mut self.source_folder).changed() {
                        self.save_config();
                    }
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.source_folder = path.display().to_string();
                            self.save_config();
                        }
                    }
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Destination Folder (where backups will be saved):");
                ui.horizontal(|ui| {
                    if ui.text_edit_singleline(&mut self.destination_folder).changed() {
                        self.save_config();
                    }
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.destination_folder = path.display().to_string();
                            self.save_config();
                        }
                    }
                });
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if !self.is_watching {
                    if ui.button("Start Watching").clicked() {
                        self.start_watching();
                    }
                } else if ui.button("Stop Watching").clicked() {
                    self.stop_watching();
                }

                if self.is_watching {
                    ui.colored_label(egui::Color32::GREEN, "● Watching");
                } else {
                    ui.colored_label(egui::Color32::GRAY, "○ Not Watching");
                }
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                let mut start_on_boot = self.start_on_boot;
                if ui.checkbox(&mut start_on_boot, "Start on system boot").changed() {
                    self.set_start_on_boot(start_on_boot);
                }
            });

            ui.add_space(20.0);

            ui.separator();
            ui.label("Status Messages:");

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for message in &self.status_messages {
                        ui.label(message);
                    }
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfig {
            start_on_boot: true,
            source_folder: "/test/source".to_string(),
            destination_folder: "/test/dest".to_string(),
        };

        // Test serialization
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("start_on_boot"));
        assert!(json.contains("true"));
        assert!(json.contains("/test/source"));
        assert!(json.contains("/test/dest"));

        // Test deserialization
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.start_on_boot, true);
        assert_eq!(deserialized.source_folder, "/test/source");
        assert_eq!(deserialized.destination_folder, "/test/dest");
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.start_on_boot, false);
        assert_eq!(config.source_folder, "");
        assert_eq!(config.destination_folder, "");
    }

    #[test]
    fn test_get_auto_launch() {
        // Test that we can create an AutoLaunch instance
        let result = get_auto_launch();
        // This should succeed as long as we can get the current exe path
        assert!(result.is_ok());
    }
}
