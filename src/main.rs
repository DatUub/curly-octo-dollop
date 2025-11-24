use eframe::egui;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "SiegeSaver - Replay File Backup Utility",
        options,
        Box::new(|_cc| Ok(Box::new(SiegeSaverApp::default()))),
    )
}

#[derive(Default)]
struct SiegeSaverApp {
    source_folder: String,
    destination_folder: String,
    watcher: Option<Arc<Mutex<Debouncer<notify::RecommendedWatcher, FileIdMap>>>>,
    status_messages: Vec<String>,
    is_watching: bool,
}

impl SiegeSaverApp {
    fn add_status(&mut self, message: String) {
        self.status_messages.push(format!("[{}] {}", 
            chrono::Local::now().format("%H:%M:%S"), 
            message
        ));
        // Keep only the last 100 messages
        if self.status_messages.len() > 100 {
            self.status_messages.remove(0);
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
            self.add_status(format!("Error: Source folder does not exist: {}", self.source_folder));
            return;
        }

        if !destination_path.exists() {
            if let Err(e) = fs::create_dir_all(&destination_path) {
                self.add_status(format!("Error creating destination folder: {}", e));
                return;
            }
            self.add_status(format!("Created destination folder: {}", self.destination_folder));
        }

        let dest_clone = destination_path.clone();
        let (tx, rx) = channel();

        let mut debouncer = match new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        for event in events {
                            if let Err(e) = tx.send(event.event) {
                                eprintln!("Error sending event: {}", e);
                            }
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            eprintln!("File watch error: {:?}", error);
                        }
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

        if let Err(e) = debouncer.watcher().watch(&source_path, RecursiveMode::Recursive) {
            self.add_status(format!("Error watching folder: {}", e));
            return;
        }

        self.watcher = Some(Arc::new(Mutex::new(debouncer)));
        self.is_watching = true;
        self.add_status(format!("Started watching: {}", self.source_folder));

        // Spawn a thread to handle file events
        let dest_for_thread = dest_clone;
        std::thread::spawn(move || {
            handle_file_events(rx, dest_for_thread);
        });
    }

    fn stop_watching(&mut self) {
        self.watcher = None;
        self.is_watching = false;
        self.add_status("Stopped watching".to_string());
    }
}

fn handle_file_events(rx: Receiver<Event>, destination_folder: PathBuf) {
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
                                        println!("Backed up: {} -> {}", 
                                            path.display(), 
                                            dest_path.display()
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("Error copying {}: {}", path.display(), e);
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SiegeSaver - Replay File Backup Utility");
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Source Folder (to watch for .rec files):");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.source_folder);
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.source_folder = path.display().to_string();
                        }
                    }
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label("Destination Folder (where backups will be saved):");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.destination_folder);
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.destination_folder = path.display().to_string();
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
