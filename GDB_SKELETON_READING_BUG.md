# BUG: get_sequence_name() Can't Read GDB Skeleton in .1aln Files

## Problem

`OneFile::get_sequence_name()` and `get_all_sequence_names()` always return `None`/empty, even when the .1aln file DOES contain GDB skeleton data with 'S' (scaffold) records.

## Evidence

### 1. The .1aln File Has Scaffolds

Using ONEview shows the data IS present:

```bash
$ ~/fastga-rs/deps/fastga/ONEview /tmp/with_gdb.1aln | grep "^[gS]" | head -20
g
S 13 SGDref#1#chrI
C 230218
S 14 SGDref#1#chrII
C 813184
S 15 SGDref#1#chrIII
C 316620
...
```

The file has:
- 136 'S' (scaffold) records
- All inside a 'g' (group) object
- Each 'S' line has a string (the sequence name)

### 2. But get_sequence_name() Returns None

```rust
let mut file = OneFile::open_read("with_gdb.1aln", Some(&schema), Some("aln"), 1)?;

// Try to get sequence 0
match file.get_sequence_name(0) {
    Some(name) => println!("Found: {}", name),
    None => println!("NOT FOUND"),  // <-- Always hits this
}

// Try to get all
let all = file.get_all_sequence_names();
println!("Count: {}", all.len());  // <-- Always returns 0
```

**Output:**
```
NOT FOUND
Count: 0
```

## Root Cause

Looking at `/home/erik/.cargo/git/checkouts/onecode-rs-ae9ceae2b231d338/21626bd/src/file.rs:534-561`:

```rust
pub fn get_sequence_name(&mut self, seq_id: i64) -> Option<String> {
    // Save current position
    let saved_line = self.line_number();

    // Go to the beginning of the file to scan for S lines
    unsafe {
        // Try to goto the start of S objects (if indexed)
        if ffi::oneGoto(self.ptr, 'S' as i8, 0) {  // <-- THIS FAILS
            let mut current_id = 0i64;
            loop {
                let line_type = ffi::oneReadLine(self.ptr) as u8 as char;
                if line_type == '\0' {
                    break;
                }
                if line_type == 'S' {
                    ...
                }
            }
        }
    }
    None  // <-- Always returns None because oneGoto failed
}
```

**The problem:** `oneGoto(self.ptr, 'S' as i8, 0)` returns `false` because:

1. 'S' is NOT a top-level object type ('O' in the schema)
2. 'S' is a GROUP member ('G' in the schema): `~ G S 0`
3. 'S' lines are INSIDE 'g' group objects
4. oneGoto() can't jump directly to 'S' records

## The Schema

From ONEview output:

```
~ O g 0                       groups scaffolds into a GDB skeleton
~ G S 0                         collection of scaffolds constituting a GDB
~ O S 1 6 STRING              id for a scaffold
```

This means:
- `g` is an **object** type (`O g 0`) - can be indexed
- `S` is a **group member** (`G S 0`) - INSIDE 'g' objects
- `S` has one STRING field - the sequence name

## Correct Implementation

To read 'S' records, we need to:

1. Navigate to the 'g' (group) object
2. Read lines until we hit 'S' records
3. Extract the STRING field from each 'S' line

```rust
pub fn get_all_sequence_names(&mut self) -> HashMap<i64, String> {
    let mut names = HashMap::new();
    let saved_line = self.line_number();

    unsafe {
        // Go to the 'g' group object (not 'S' directly!)
        if ffi::oneGoto(self.ptr, 'g' as i8, 0) {
            let mut current_id = 0i64;

            loop {
                let line_type = ffi::oneReadLine(self.ptr) as u8 as char;

                if line_type == '\0' {
                    break; // EOF
                }

                if line_type == 'S' {
                    // 'S' has one STRING field
                    if let Some(name) = self.string() {
                        names.insert(current_id, name.to_string());
                        current_id += 1;
                    }
                }

                // Stop when we hit the next 'g' or reach 'A' (alignments)
                if line_type == 'g' || line_type == 'A' {
                    break;
                }
            }

            // Restore position
            let _ = ffi::oneGoto(self.ptr, (*self.ptr).lineType, saved_line);
        }
    }

    names
}
```

## Testing the Fix

Once fixed, this should work:

```rust
let mut file = OneFile::open_read("with_gdb.1aln", Some(&schema), Some("aln"), 1)?;

let all_names = file.get_all_sequence_names();
assert_eq!(all_names.len(), 136); // 136 scaffolds in test file

// Check specific names
assert_eq!(all_names.get(&0), Some(&"SGDref#1#chrI".to_string()));
assert_eq!(all_names.get(&1), Some(&"SGDref#1#chrII".to_string()));

// Individual lookup
let name = file.get_sequence_name(0)?;
assert_eq!(name, "SGDref#1#chrI");
```

## Priority

**CRITICAL** - This is the final blocker for pure .1aln filtering in sweepga.

Without this fix:
- ❌ Can't get real sequence names from .1aln
- ❌ Must convert .1aln → PAF to get names
- ❌ Can't group alignments by chromosome pairs properly

With this fix:
- ✅ Pure .1aln filtering works
- ✅ No PAF intermediate needed
- ✅ Sweepga can use X-field identity directly

## Files to Fix

1. `/home/erik/.cargo/git/checkouts/onecode-rs-ae9ceae2b231d338/21626bd/src/file.rs`
   - Line 534: `get_sequence_name()`
   - Line 570: `get_all_sequence_names()`

Both need to navigate to 'g' object first, then scan for 'S' lines.

## Additional Note

The same issue likely affects other group members in ONE files. The pattern of "navigate to parent object, then scan for group members" should be documented for future API users.
