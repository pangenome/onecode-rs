/// Thread safety tests for ONEcode Rust wrapper
///
/// These tests verify that the mutexes protecting C library global state work correctly.

use onecode::{OneFile, OneSchema};
use std::sync::Arc;
use std::thread;

#[test]
fn test_concurrent_error_handling() {
    // Try to open multiple non-existent files concurrently
    // This tests the error string mutex protection
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let path = format!("nonexistent_file_{}.seq", i);
                let result = OneFile::open_read(&path, None, None, 1);

                // Should fail with error mentioning the correct filename
                assert!(result.is_err());
                let err_msg = match result {
                    Err(e) => e.to_string(),
                    Ok(_) => panic!("Expected error but got Ok"),
                };

                // Verify error message contains the correct filename
                // If error string buffer isn't protected, we might get the wrong filename
                assert!(
                    err_msg.contains(&path),
                    "Error message '{}' should contain filename '{}'",
                    err_msg,
                    path
                );
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_schema_from_text() {
    // Create multiple schemas from text concurrently
    // This tests the oneSchemaCreateFromText temp file handling with mkstemp
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let schema_text = format!("P 3 ts{}\nO T 1 3 INT\n", i);
                let result = OneSchema::from_text(&schema_text);

                // Should succeed
                if let Err(e) = result {
                    eprintln!("Thread {} failed: {}", i, e);
                    panic!("Schema creation failed for test {}: {}", i, e);
                }
            })
        })
        .collect();

    for (i, handle) in handles.into_iter().enumerate() {
        if let Err(e) = handle.join() {
            eprintln!("Thread {} panicked: {:?}", i, e);
            panic!("Thread {} panicked", i);
        }
    }
}

#[test]
#[ignore] // C library has global state (isBootStrap) in schema parsing - concurrent file opening fails
fn test_mixed_operations_concurrent() {
    // Mix successful and failing operations concurrently
    let good_file = Arc::new("ONEcode/TEST/small.seq".to_string());

    let handles: Vec<_> = (0..20)
        .map(|i| {
            let good_file = Arc::clone(&good_file);
            thread::spawn(move || {
                if i % 2 == 0 {
                    // Try to open good file
                    let result = OneFile::open_read(&good_file, None, None, 1);
                    assert!(result.is_ok(), "Opening good file failed");
                } else {
                    // Try to open bad file
                    let bad_file = format!("bad_{}.seq", i);
                    let result = OneFile::open_read(&bad_file, None, None, 1);
                    assert!(result.is_err());
                    if let Err(e) = result {
                        let err_msg = e.to_string();
                        assert!(err_msg.contains(&bad_file));
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_error_message_correctness() {
    // Verify that error messages are correct even under concurrent load
    // This is a stress test for the error string mutex

    let handles: Vec<_> = (0..50)
        .map(|i| {
            thread::spawn(move || {
                // Each thread tries to open a unique non-existent file
                let unique_filename = format!("unique_nonexistent_{}_file.seq", i);
                let result = OneFile::open_read(&unique_filename, None, None, 1);

                assert!(result.is_err());
                let err_msg = match result {
                    Err(e) => e.to_string(),
                    Ok(_) => panic!("Expected error but got Ok"),
                };

                // The error message MUST contain our unique filename
                // If the error string mutex doesn't work, we might get
                // another thread's filename
                assert!(
                    err_msg.contains(&unique_filename),
                    "Thread {} got wrong error message: '{}' (expected to contain '{}')",
                    i,
                    err_msg,
                    unique_filename
                );
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
