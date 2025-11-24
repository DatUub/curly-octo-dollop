use notify::{EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::fs;
use std::sync::mpsc::channel;
use std::time::Duration;

#[test]
fn test_file_watcher_detects_rec_files() {
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

    // Spawn a thread to handle file events
    let handler = std::thread::spawn(move || {
        let mut copied_files = Vec::new();
        let start = std::time::Instant::now();

        // Listen for events for up to 2 seconds
        while start.elapsed() < Duration::from_secs(2) {
            if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if let Some(extension) = path.extension() {
                                if extension == "rec" {
                                    if let Some(filename) = path.file_name() {
                                        let dest_path = dest_clone.join(filename);
                                        if fs::copy(&path, &dest_path).is_ok() {
                                            copied_files
                                                .push(filename.to_string_lossy().to_string());
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
        copied_files
    });

    // Give the watcher a moment to start
    std::thread::sleep(Duration::from_millis(200));

    // Create test .rec files
    let test_file1 = source_dir.join("test1.rec");
    let test_file2 = source_dir.join("test2.rec");
    let test_file_other = source_dir.join("test.txt");

    fs::write(&test_file1, "test content 1").expect("Failed to write test file 1");
    fs::write(&test_file2, "test content 2").expect("Failed to write test file 2");
    fs::write(&test_file_other, "should not be copied").expect("Failed to write test file 3");

    // Wait for the handler to finish
    let copied_files = handler.join().expect("Handler thread panicked");

    // Verify that .rec files were copied
    assert!(
        copied_files.contains(&"test1.rec".to_string()),
        "test1.rec should have been copied"
    );
    assert!(
        copied_files.contains(&"test2.rec".to_string()),
        "test2.rec should have been copied"
    );

    // Verify files exist in destination
    assert!(
        dest_dir.join("test1.rec").exists(),
        "test1.rec should exist in destination"
    );
    assert!(
        dest_dir.join("test2.rec").exists(),
        "test2.rec should exist in destination"
    );

    // Verify non-.rec file was not copied
    assert!(
        !dest_dir.join("test.txt").exists(),
        "test.txt should not be copied"
    );

    // Test persistence: delete source files
    fs::remove_file(&test_file1).expect("Failed to delete source file 1");
    fs::remove_file(&test_file2).expect("Failed to delete source file 2");

    // Verify destination files still exist
    assert!(
        dest_dir.join("test1.rec").exists(),
        "test1.rec should still exist in destination after source deletion"
    );
    assert!(
        dest_dir.join("test2.rec").exists(),
        "test2.rec should still exist in destination after source deletion"
    );

    // Clean up
    fs::remove_dir_all(&test_dir).expect("Failed to clean up test directory");
}
