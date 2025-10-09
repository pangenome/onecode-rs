/// Test to reproduce schema temporary file cleanup bug
///
/// Run with: cargo test --test schema_parallel_test
/// Expected: Both tests should pass when run together

use onecode::{OneSchema, OneFile};

const TEST_SCHEMA: &str = r#"
P 3 tst  Test schema for parallel creation
O A 3 3 INT 3 INT 3 INT
D B 1 3 INT
"#;

#[test]
fn test_schema_with_file_1() {
    let schema = OneSchema::from_text(TEST_SCHEMA).expect("Failed to create schema 1");
    let path = format!("/tmp/test_schema_1_{}.tst", std::process::id());

    let file = OneFile::open_write_new(&path, &schema, "tst", true, 1)
        .expect("Failed to open file 1");
    file.close();
    let _ = std::fs::remove_file(&path);
    println!("✓ Test 1 completed");
}

#[test]
fn test_schema_with_file_2() {
    let schema = OneSchema::from_text(TEST_SCHEMA).expect("Failed to create schema 2");
    let path = format!("/tmp/test_schema_2_{}.tst", std::process::id());

    let file = OneFile::open_write_new(&path, &schema, "tst", true, 1)
        .expect("Failed to open file 2");
    file.close();
    let _ = std::fs::remove_file(&path);
    println!("✓ Test 2 completed");
}

#[test]
fn test_schema_with_file_3() {
    let schema = OneSchema::from_text(TEST_SCHEMA).expect("Failed to create schema 3");
    let path = format!("/tmp/test_schema_3_{}.tst", std::process::id());

    let file = OneFile::open_write_new(&path, &schema, "tst", true, 1)
        .expect("Failed to open file 3");
    file.close();
    let _ = std::fs::remove_file(&path);
    println!("✓ Test 3 completed");
}

#[test]
fn test_schema_with_file_4() {
    let schema = OneSchema::from_text(TEST_SCHEMA).expect("Failed to create schema 4");
    let path = format!("/tmp/test_schema_4_{}.tst", std::process::id());

    let file = OneFile::open_write_new(&path, &schema, "tst", true, 1)
        .expect("Failed to open file 4");
    file.close();
    let _ = std::fs::remove_file(&path);
    println!("✓ Test 4 completed");
}

#[test]
fn test_schema_with_file_5() {
    let schema = OneSchema::from_text(TEST_SCHEMA).expect("Failed to create schema 5");
    let path = format!("/tmp/test_schema_5_{}.tst", std::process::id());

    let file = OneFile::open_write_new(&path, &schema, "tst", true, 1)
        .expect("Failed to open file 5");
    file.close();
    let _ = std::fs::remove_file(&path);
    println!("✓ Test 5 completed");
}
