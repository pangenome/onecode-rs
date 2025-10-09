use onecode::OneFile;

#[test]
fn test_get_all_sequence_names() {
    let mut file = OneFile::open_read("data/test.1aln", None, None, 1)
        .expect("Failed to open test.1aln");

    let names = file.get_all_sequence_names();

    println!("Found {} sequences:", names.len());
    for (id, name) in &names {
        println!("  {}: {}", id, name);
    }

    // Verify we got some sequences
    assert!(!names.is_empty(), "Should have found sequence names");

    // Check specific sequences based on the ONEview output
    assert!(names.contains_key(&0), "Should have sequence 0");
    assert!(names.contains_key(&1), "Should have sequence 1");

    // Verify the first sequence name contains expected content
    let seq0 = names.get(&0).expect("Sequence 0 should exist");
    assert!(
        seq0.contains("gi|568815592"),
        "Sequence 0 should contain gi|568815592, got: {}",
        seq0
    );

    let seq1 = names.get(&1).expect("Sequence 1 should exist");
    assert!(
        seq1.contains("gi|568815529"),
        "Sequence 1 should contain gi|568815529, got: {}",
        seq1
    );
}

#[test]
fn test_get_sequence_name_individual() {
    let mut file = OneFile::open_read("data/test.1aln", None, None, 1)
        .expect("Failed to open test.1aln");

    // Get sequence 0
    let name0 = file
        .get_sequence_name(0)
        .expect("Should find sequence 0");
    println!("Sequence 0: {}", name0);
    assert!(name0.contains("gi|568815592"));

    // Get sequence 1
    let name1 = file
        .get_sequence_name(1)
        .expect("Should find sequence 1");
    println!("Sequence 1: {}", name1);
    assert!(name1.contains("gi|568815529"));

    // Get sequence 5
    let name5 = file
        .get_sequence_name(5)
        .expect("Should find sequence 5");
    println!("Sequence 5: {}", name5);
    assert!(name5.contains("gi|568815569"));

    // Non-existent sequence
    let name_invalid = file.get_sequence_name(999);
    assert!(name_invalid.is_none(), "Should return None for invalid ID");
}

#[test]
fn test_alignment_with_sequence_names() {
    let mut file = OneFile::open_read("data/test.1aln", None, None, 1)
        .expect("Failed to open test.1aln");

    // Read through alignments and look up sequence names
    let mut alignment_count = 0;
    loop {
        let line_type = file.read_line();
        if line_type == '\0' {
            break;
        }

        if line_type == 'A' {
            // A lines have format: A <a_id> <a_start> <a_len> <b_id> <b_start> <b_len>
            let a_id = file.int(0);
            let b_id = file.int(3);

            if let (Some(a_name), Some(b_name)) =
                (file.get_sequence_name(a_id), file.get_sequence_name(b_id)) {
                println!("Alignment {}: {} vs {}", alignment_count, a_name, b_name);
                alignment_count += 1;

                // Just test the first few to avoid too much output
                if alignment_count >= 3 {
                    break;
                }
            }
        }
    }

    assert!(alignment_count > 0, "Should have found some alignments");
}
