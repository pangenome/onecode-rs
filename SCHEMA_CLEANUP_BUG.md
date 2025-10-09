# Bug Report: Schema Temporary File Cleanup Issue

**Status**: ⚠️ **WORKAROUND IMPLEMENTED** (serialization) - upstream fix still recommended

## Problem

When running multiple Rust tests that create `OneSchema` instances from text, tests fail with:

```
FATAL ERROR: OneFile schema error; multiple list types for linetype definition &
```

Or alternatively:

```
FATAL ERROR: failed to remove temporary file /tmp/OneTextSchema-XXXXXX.schema errno 2
```

## Symptoms

- ✅ **Individual tests pass**: Each test works perfectly when run alone
- ❌ **Parallel tests fail**: Running multiple tests together fails
- 🐛 **Root cause**: Temporary `.schema` files in `/tmp/` conflict between tests

## Reproduction

**Test code** (from `fastga-rs/src/onelib.rs`):

```rust
fn create_aln_schema() -> Result<OneSchema> {
    let schema_text = r#"
P 3 aln
O A 6 3 INT 3 INT 3 INT 3 INT 3 INT 3 INT
D L 2 3 INT 3 INT
D R 0
D Q 1 3 INT
D M 1 3 INT
D D 1 3 INT
D C 1 6 STRING
D T 1 8 INT_LIST
D X 1 8 INT_LIST
D p 2 3 INT 3 INT
O a 1 3 INT
G A 0
"#;
    OneSchema::from_text(schema_text)
        .context("Failed to create .1aln schema")
}

#[test]
fn test_write_simple_alignment() {
    let schema = create_aln_schema().unwrap();  // Works fine
    let file = OneFile::open_write_new("test.1aln", &schema, "aln", true, 1).unwrap();
    // ... write data ...
}

#[test]
fn test_roundtrip() {
    let schema = create_aln_schema().unwrap();  // Conflicts with first test!
    // ... FATAL ERROR here when both tests run
}
```

**How to reproduce:**

```bash
cd ~/fastga-rs
cargo test --lib                     # ❌ FAILS with schema error
cargo test --lib test_roundtrip      # ✅ PASSES when run alone
cargo test --lib test_write_simple   # ✅ PASSES when run alone
cargo test --lib -- --test-threads=1 # ✅ PASSES with serial execution
```

## Analysis

Looking at the C code in `ONEcode/ONElib.c`, `OneSchema::from_text()` calls:

```c
OneSchema *oneSchemaCreateFromText (char *text)
{
  static int schemaFileCount = 0;
  char *filename = Sprintf("/tmp/OneTextSchema-%d.schema", schemaFileCount++);
  FILE *f = fopen(filename, "w");
  fprintf(f, "%s", text);
  fclose(f);

  OneSchema *schema = oneSchemaCreateFromFile(filename);

  remove(filename);  // ← This cleanup appears to fail sometimes!
  free(filename);
  return schema;
}
```

**Issues:**
1. **Race condition**: Multiple threads/tests can create schemas with the same counter
2. **Failed cleanup**: `remove(filename)` returns errno 2 (ENOENT) - file already deleted?
3. **Global state**: `static int schemaFileCount` is process-wide, not thread-safe

## Expected Behavior

1. Each schema creation should use a unique temporary file (no conflicts)
2. Temporary files should be cleaned up reliably
3. Multiple tests should be able to create schemas concurrently

## Workaround

### ✅ Working Solution (Implemented in fastga-rs)

Serialize schema creation using a global Mutex:

```rust
use std::sync::Mutex;

/// Global lock to serialize schema creation
static SCHEMA_CREATION_LOCK: Mutex<()> = Mutex::new(());

fn create_aln_schema() -> Result<OneSchema> {
    // Hold lock during schema creation to serialize temp file operations
    let _guard = SCHEMA_CREATION_LOCK.lock().unwrap();

    let schema_text = r#"P 3 aln
O A 6 3 INT 3 INT 3 INT 3 INT 3 INT 3 INT
..."#;

    OneSchema::from_text(schema_text)
        .context("Failed to create .1aln schema")
}
```

**Status**: ✅ Tests now pass reliably in parallel (fastga-rs commit 50fb1b1)

### ❌ Doesn't Work: Serial Test Execution

Running tests serially works but is slow:
```bash
cargo test -- --test-threads=1
```

## Suggested Fixes

### Option 1: Make schema counter thread-safe
```c
#include <stdatomic.h>
static atomic_int schemaFileCount = 0;
char *filename = Sprintf("/tmp/OneTextSchema-%d-%d.schema",
                         getpid(), atomic_fetch_add(&schemaFileCount, 1));
```

### Option 2: Use mkstemp() for guaranteed unique files
```c
char template[] = "/tmp/OneTextSchema-XXXXXX.schema";
int fd = mkstemps(template, 7);  // 7 = strlen(".schema")
FILE *f = fdopen(fd, "w");
// ... write schema ...
fclose(f);
OneSchema *schema = oneSchemaCreateFromFile(template);
unlink(template);
```

### Option 3: Cache schemas by content hash
```c
static GHashTable *schema_cache = NULL;  // hash(text) -> OneSchema*
// If schema already created from this text, return cached version
```

### Option 4: Add cleanup to Drop/destructor
Make sure schemas properly clean up their temp files when dropped, even if creation fails.

## Environment

- OS: Linux 6.16.0
- onecode-rs: commit f18b3fe0
- Rust: 1.82+
- fastga-rs: commit c5c7731

## Request

Can you investigate and fix this? It's blocking parallel test execution in fastga-rs (which uses onecode-rs for .1aln I/O).

**Priority:** Medium-High (tests work but only serially)

cc: @pangenome/onecode-rs
