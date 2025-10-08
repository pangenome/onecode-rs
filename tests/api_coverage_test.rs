/// Test coverage for all ONE file API methods
///
/// This test demonstrates all the macros from ONElib.h exposed as Rust methods

use onecode::{OneFile, OneSchema};

#[test]
fn test_all_field_accessors() {
    // Create a simple schema with different field types
    let schema_text = "P 3 tst\nO T 3 3 INT 4 REAL 4 CHAR\n";
    let schema = OneSchema::from_text(schema_text).unwrap();

    let path = "/tmp/test_api_coverage.1tst";
    let mut writer = OneFile::open_write_new(path, &schema, "tst", false, 1).unwrap();

    // Test setting fields (oneInt, oneReal, oneChar)
    writer.set_int(0, 42);
    writer.set_real(1, 3.14);
    writer.set_char(2, 'X');
    writer.write_line('T', 0, None);

    drop(writer);

    // Read back and test field accessors
    let mut reader = OneFile::open_read(path, None, None, 1).unwrap();

    // Test oneFileName
    assert!(reader.file_name().unwrap().contains("test_api_coverage"));

    reader.read_line();

    // Test oneInt, oneReal, oneChar
    assert_eq!(reader.int(0), 42);
    assert!((reader.real(1) - 3.14).abs() < 0.001);
    assert_eq!(reader.char(2), 'X');

    std::fs::remove_file(path).ok();
}

#[test]
fn test_list_accessors() {
    // Schema with integer list
    let schema_text = "P 3 tst\nO L 1 8 INT_LIST\n";
    let schema = OneSchema::from_text(schema_text).unwrap();

    let path = "/tmp/test_list_api.1tst";
    let mut writer = OneFile::open_write_new(path, &schema, "tst", false, 1).unwrap();

    // Write an integer list
    let data: Vec<i64> = vec![10, 20, 30, 40, 50];
    writer.write_line('L', data.len() as i64, Some(data.as_ptr() as *mut std::ffi::c_void));

    drop(writer);

    // Read back and test list accessors
    let mut reader = OneFile::open_read(path, None, None, 1).unwrap();
    reader.read_line();

    // Test oneLen
    assert_eq!(reader.len(), 5);

    // Test is_empty
    assert!(!reader.is_empty());

    // Test oneIntList
    let list = reader.int_list().unwrap();
    assert_eq!(list, &[10, 20, 30, 40, 50]);

    std::fs::remove_file(path).ok();
}

#[test]
fn test_string_accessors() {
    // Schema with string
    let schema_text = "P 3 tst\nO S 1 6 STRING\n";
    let schema = OneSchema::from_text(schema_text).unwrap();

    let path = "/tmp/test_string_api.1tst";
    let mut writer = OneFile::open_write_new(path, &schema, "tst", false, 1).unwrap();

    let test_str = "Hello, ONEcode!";
    writer.write_line('S', test_str.len() as i64, Some(test_str.as_ptr() as *mut std::ffi::c_void));

    drop(writer);

    // Read back and test string accessor
    let mut reader = OneFile::open_read(path, None, None, 1).unwrap();
    reader.read_line();

    // Test oneString
    let s = reader.string().unwrap();
    assert_eq!(s, test_str);

    // Test oneLen for string
    assert_eq!(reader.len(), test_str.len() as i64);

    std::fs::remove_file(path).ok();
}

#[test]
fn test_object_and_reference_count() {
    let schema_text = "P 3 seq\nO S 1 3 DNA\n";
    let schema = OneSchema::from_text(schema_text).unwrap();

    let path = "/tmp/test_counts.1seq";
    let mut writer = OneFile::open_write_new(path, &schema, "seq", false, 1).unwrap();

    // Add a reference
    writer.add_reference("test.fa", 100).unwrap();

    // Write some sequences
    writer.write_line('S', 0, None);
    writer.write_line('S', 0, None);

    drop(writer);

    // Read back and test counters
    let mut reader = OneFile::open_read(path, None, None, 1).unwrap();

    // Read through the file to populate counts
    while reader.read_line() != '\0' {}

    // Test oneObject - get object count for line type 'S'
    let s_count = reader.object('S');
    assert_eq!(s_count, 2);

    // Test oneReferenceCount
    let ref_count = reader.reference_count();
    assert_eq!(ref_count, 1);

    std::fs::remove_file(path).ok();
}

#[test]
fn test_real_list() {
    // Schema with real list
    let schema_text = "P 3 tst\nO R 1 9 REAL_LIST\n";
    let schema = OneSchema::from_text(schema_text).unwrap();

    let path = "/tmp/test_real_list.1tst";
    let mut writer = OneFile::open_write_new(path, &schema, "tst", false, 1).unwrap();

    // Write a real list
    let data: Vec<f64> = vec![1.1, 2.2, 3.3, 4.4];
    writer.write_line('R', data.len() as i64, Some(data.as_ptr() as *mut std::ffi::c_void));

    drop(writer);

    // Read back and test real list accessor
    let mut reader = OneFile::open_read(path, None, None, 1).unwrap();
    reader.read_line();

    // Test oneRealList
    let list = reader.real_list().unwrap();
    assert_eq!(list.len(), 4);
    assert!((list[0] - 1.1).abs() < 0.001);
    assert!((list[3] - 4.4).abs() < 0.001);

    std::fs::remove_file(path).ok();
}
