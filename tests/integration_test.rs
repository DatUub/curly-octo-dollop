use notify::{EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::fs;
use std::sync::mpsc::channel;
use std::time::Duration;

#[test]
fn test_file_watcher_detects_directories() {
    // Create temporary test directories
    let test_dir = std::env::temp_dir().join("siegesaver_test");
    let source_dir = test_dir.join("source");
    let dest_dir = test_dir.join("dest");

    // Clean up any previous test runs
    let _ = fs::remove_dir_all(&test_dir);

    fs::create_dir_all(&source_dir).expect("Failed to create source directory");
    fs::create_dir_all(&dest_dir).expect("Failed to create dest directory");

    let (tx, rx) = channel();
    let dest_clone = dest_dir.clone();

    // Create debouncer with shorter timeout for testing
    let mut debouncer = new_debouncer(
        Duration::from_millis(100),
        None,
        move |result: DebounceEventResult| {
            if let Ok(events) = result {
                for event in events {
                    let _ = tx.send(event.event);
                }
            }
        },
    )
    .expect("Failed to create debouncer");

    debouncer
        .watcher()
        .watch(&source_dir, RecursiveMode::Recursive)
        .expect("Failed to watch source directory");

    // Helper function to recursively copy a directory (mirroring main.rs implementation)
    fn copy_directory_recursive(
        source: &std::path::PathBuf,
        destination: &std::path::PathBuf,
    ) -> std::io::Result<()> {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let dest_path = destination.join(&file_name);
            if path.is_dir() {
                copy_directory_recursive(&path, &dest_path)?;
            } else {
                fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    // Spawn a thread to handle directory events
    let handler = std::thread::spawn(move || {
        let mut copied_folders = Vec::new();
        let start = std::time::Instant::now();

        // Listen for events for up to 2 seconds
        while start.elapsed() < Duration::from_secs(2) {
            if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                match event.kind {
                    EventKind::Create(_) => {
                        for path in event.paths {
                            // Check if the path is a directory
                            if path.is_dir() {
                                if let Some(folder_name) = path.file_name() {
                                    let dest_path = dest_clone.join(folder_name);

                                    // Skip if destination already exists
                                    if dest_path.exists() {
                                        continue;
                                    }

                                    // Copy the entire directory recursively
                                    if copy_directory_recursive(&path, &dest_path).is_ok() {
                                        copied_folders
                                            .push(folder_name.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        copied_folders
    });

    // Give the watcher a moment to start
    std::thread::sleep(Duration::from_millis(200));

    // Create test match folders with files inside
    let match_folder1 = source_dir.join("Match-2025-11-23-001");
    let match_folder2 = source_dir.join("Match-2025-11-23-002");

    fs::create_dir_all(&match_folder1).expect("Failed to create match folder 1");
    fs::create_dir_all(&match_folder2).expect("Failed to create match folder 2");

    // Create some .rec files inside the match folders
    fs::write(match_folder1.join("replay1.rec"), "replay content 1")
        .expect("Failed to write replay file 1");
    fs::write(match_folder1.join("replay2.rec"), "replay content 2")
        .expect("Failed to write replay file 2");
    fs::write(match_folder2.join("replay3.rec"), "replay content 3")
        .expect("Failed to write replay file 3");

    // Wait for the handler to finish
    let copied_folders = handler.join().expect("Handler thread panicked");

    // Verify that match folders were copied
    assert!(
        copied_folders.contains(&"Match-2025-11-23-001".to_string()),
        "Match-2025-11-23-001 should have been copied"
    );
    assert!(
        copied_folders.contains(&"Match-2025-11-23-002".to_string()),
        "Match-2025-11-23-002 should have been copied"
    );

    // Verify folders exist in destination with their contents
    let dest_match1 = dest_dir.join("Match-2025-11-23-001");
    let dest_match2 = dest_dir.join("Match-2025-11-23-002");

    assert!(
        dest_match1.exists(),
        "Match-2025-11-23-001 should exist in destination"
    );
    assert!(
        dest_match2.exists(),
        "Match-2025-11-23-002 should exist in destination"
    );

    assert!(
        dest_match1.join("replay1.rec").exists(),
        "replay1.rec should exist in destination folder"
    );
    assert!(
        dest_match1.join("replay2.rec").exists(),
        "replay2.rec should exist in destination folder"
    );
    assert!(
        dest_match2.join("replay3.rec").exists(),
        "replay3.rec should exist in destination folder"
    );

    // Test persistence: delete source folders
    fs::remove_dir_all(&match_folder1).expect("Failed to delete source folder 1");
    fs::remove_dir_all(&match_folder2).expect("Failed to delete source folder 2");

    // Verify destination folders still exist
    assert!(
        dest_match1.exists(),
        "Match-2025-11-23-001 should still exist in destination after source deletion"
    );
    assert!(
        dest_match2.exists(),
        "Match-2025-11-23-002 should still exist in destination after source deletion"
    );
    assert!(
        dest_match1.join("replay1.rec").exists(),
        "replay1.rec should still exist after source deletion"
    );

    // Clean up
    fs::remove_dir_all(&test_dir).expect("Failed to clean up test directory");
}
