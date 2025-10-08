/// Example: Read a sequence file and print statistics
///
/// Usage: cargo run --example read_seq -- ONEcode/TEST/small.seq

use onecode::{OneFile, Result};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.seq>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    println!("Reading file: {}", filename);

    // Open the file
    let mut file = OneFile::open_read(filename, None, None, 1)?;

    // Print file metadata
    println!("File type: {:?}", file.file_type());
    println!("File name: {:?}", file.file_name());

    // Get statistics
    if let Ok((count, max, total)) = file.stats('S') {
        println!("\nSequence statistics:");
        println!("  Count: {}", count);
        println!("  Longest sequence: {} bp", max);
        println!("  Total sequence: {} bp", total);
    }

    if let Ok((count, max, total)) = file.stats('I') {
        println!("\nIdentifier statistics:");
        println!("  Count: {}", count);
        println!("  Longest ID: {} chars", max);
        println!("  Total: {} chars", total);
    }

    // Read through the file
    println!("\nReading data:");
    let mut seq_count = 0;
    let mut id_count = 0;

    loop {
        let line_type = file.read_line();
        if line_type == '\0' {
            break;
        }

        match line_type {
            'S' => {
                seq_count += 1;
                println!("  Line {}: Sequence (type S)", file.line_number());
            }
            'I' => {
                id_count += 1;
                println!("  Line {}: ID (type I)", file.line_number());
            }
            _ => {
                println!("  Line {}: Other type '{}'", file.line_number(), line_type);
            }
        }
    }

    println!("\nRead {} sequences and {} identifiers", seq_count, id_count);

    Ok(())
}
