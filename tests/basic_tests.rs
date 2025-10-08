use onecode::{OneFile, OneSchema, Result};

#[test]
fn test_open_read_simple_seq() -> Result<()> {
    let mut file = OneFile::open_read("ONEcode/TEST/small.seq", None, None, 1)?;

    // Check file metadata
    assert_eq!(file.file_type(), Some("seq".to_string()));

    // Read through the file
    let mut s_count = 0;
    let mut i_count = 0;

    loop {
        let line_type = file.read_line();
        if line_type == '\0' {
            break;
        }

        match line_type {
            'S' => s_count += 1,
            'I' => i_count += 1,
            _ => {}
        }
    }

    // small.seq should have 10 S lines and 10 I lines
    assert_eq!(s_count, 10);
    assert_eq!(i_count, 10);

    Ok(())
}

#[test]
fn test_open_read_foo() -> Result<()> {
    let mut file = OneFile::open_read("ONEcode/TEST/t1.foo", None, None, 1)?;

    // Check file metadata
    assert_eq!(file.file_type(), Some("foo".to_string()));

    // Read the single line
    let line_type = file.read_line();
    assert_eq!(line_type, 'B');

    // Read the integer field
    let val = file.int(0);
    assert_eq!(val, 5);

    // Should be at end of file now
    let line_type = file.read_line();
    assert_eq!(line_type, '\0');

    Ok(())
}

#[test]
fn test_stats() -> Result<()> {
    let file = OneFile::open_read("ONEcode/TEST/small.seq", None, None, 1)?;

    // Check stats for S lines (sequences)
    let (s_count, s_max, s_total) = file.stats('S')?;
    assert_eq!(s_count, 10); // 10 sequences
    assert_eq!(s_max, 72);    // longest sequence is 72 bases
    assert_eq!(s_total, 577); // total of 577 bases

    // Check stats for I lines (identifiers)
    let (i_count, i_max, i_total) = file.stats('I')?;
    assert_eq!(i_count, 10); // 10 identifiers
    assert_eq!(i_max, 5);     // longest identifier is 5 characters ("seq10")
    assert_eq!(i_total, 41);  // total of 41 characters

    Ok(())
}

#[test]
#[ignore] // TODO: C library has issue with temporary file cleanup
fn test_schema_from_text() -> Result<()> {
    let schema_text = "P 3 seq\nO S 1 3 DNA\nD I 1 6 STRING\n";
    let _schema = OneSchema::from_text(schema_text)?;

    // Just verify we can create the schema without errors
    Ok(())
}

#[test]
fn test_read_with_type_check() -> Result<()> {
    // Open file and verify it's of type "seq"
    let file = OneFile::open_read("ONEcode/TEST/small.seq", None, Some("seq"), 1)?;

    assert_eq!(file.file_type(), Some("seq".to_string()));

    Ok(())
}

#[test]
fn test_file_properties() -> Result<()> {
    let file = OneFile::open_read("ONEcode/TEST/small.seq", None, None, 1)?;

    // Check that we can read basic properties
    assert!(file.file_name().is_some());
    assert_eq!(file.file_type(), Some("seq".to_string()));

    Ok(())
}

#[test]
fn test_open_nonexistent_file() {
    // This should fail
    let result = OneFile::open_read("nonexistent.seq", None, None, 1);
    assert!(result.is_err());
}

#[test]
fn test_sequential_read() -> Result<()> {
    let mut file = OneFile::open_read("ONEcode/TEST/t2.seq", None, None, 1)?;

    // First line should be 's' (scaffold object)
    let line_type = file.read_line();
    assert_eq!(line_type, 's');

    // Next should be 'n'
    let line_type = file.read_line();
    assert_eq!(line_type, 'n');
    assert_eq!(file.int(0), 2);

    // Next should be 'S'
    let line_type = file.read_line();
    assert_eq!(line_type, 'S');

    Ok(())
}

#[test]
#[ignore] // TODO: C library has issue with temporary file cleanup in oneSchemaCreateFromText
fn test_write_and_read_roundtrip() -> Result<()> {
    use std::fs;

    // Create a schema
    let schema_text = "P 3 tst\nO T 1 3 INT\n";
    let schema = OneSchema::from_text(schema_text)?;

    // Write a file
    let output_path = "tests/test_output.1tst";
    {
        let mut writer = OneFile::open_write_new(output_path, &schema, "tst", false, 1)?;

        // Add provenance
        writer.add_provenance("test", "1.0", "test command")?;

        // Write a line
        writer.set_int(0, 42);
        writer.write_line('T', 0, None);

        writer.set_int(0, 100);
        writer.write_line('T', 0, None);

        // Explicitly close to ensure flush
        writer.close();
    }

    // Read it back
    {
        let mut reader = OneFile::open_read(output_path, None, None, 1)?;

        assert_eq!(reader.file_type(), Some("tst".to_string()));

        let line_type = reader.read_line();
        assert_eq!(line_type, 'T');
        assert_eq!(reader.int(0), 42);

        let line_type = reader.read_line();
        assert_eq!(line_type, 'T');
        assert_eq!(reader.int(0), 100);

        let line_type = reader.read_line();
        assert_eq!(line_type, '\0'); // EOF
    }

    // Clean up
    fs::remove_file(output_path).ok();

    Ok(())
}
