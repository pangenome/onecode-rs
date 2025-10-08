# Thread Safety Issues in ONEcode C Library

## Overview

The ONEcode C library has several thread safety issues that affect concurrent usage within a single process. This document describes these issues and the workarounds implemented in the Rust wrapper.

## Issue 1: `oneSchemaCreateFromText()` Race Condition

### The Problem

The `oneSchemaCreateFromText()` function has a critical race condition when called from multiple threads simultaneously within the same process.

### Root Cause

From `ONElib.c` lines 1450-1470 (approximately):

```c
OneSchema *oneSchemaCreateFromText (const char *text)
{
  static char template[64];
  sprintf(template, "/tmp/OneTextSchema-%d.schema", getpid());

  errno = 0;
  FILE *f = fopen(template, "w");
  // ... write schema text to temp file ...
  fclose(f);

  OneSchema *vs = oneSchemaCreateFromFile(template);

  // Later in oneSchemaCreateFromFile or oneSchemaDestroy:
  if (remove(template))
    die("failed to remove temporary file %s errno %d\n", template, errno);

  return vs;
}
```

**The bug**: The temp file path is based only on the process ID (`getpid()`). When multiple threads in the same process call this function:

1. Thread A creates `/tmp/OneTextSchema-12345.schema`
2. Thread B creates `/tmp/OneTextSchema-12345.schema` (same file!)
3. Thread A reads and deletes the file
4. Thread B tries to delete the file → **FATAL ERROR: errno 2 (ENOENT)**

### Symptoms

```
FATAL ERROR: failed to remove temporary file /tmp/OneSchema.XXXXXX errno 2
```

This occurs when running tests in parallel (default cargo test behavior).

### Rust Wrapper Solution

We added a global mutex in `src/schema.rs`:

```rust
static SCHEMA_FROM_TEXT_LOCK: Mutex<()> = Mutex::new(());

pub fn from_text(text: &str) -> Result<Self> {
    let _guard = SCHEMA_FROM_TEXT_LOCK.lock().unwrap();
    // ... call C function ...
}
```

This serializes all calls to `oneSchemaCreateFromText()` within the process, preventing the race condition.

### Recommended Fix for C Library

Replace the process ID with thread-safe unique identifier:

```c
// Option 1: Use thread ID + counter
static _Atomic int counter = 0;
sprintf(template, "/tmp/OneTextSchema-%d-%d-%d.schema",
        getpid(), gettid(), atomic_fetch_add(&counter, 1));

// Option 2: Use mkstemp()
char template[] = "/tmp/OneTextSchema-XXXXXX.schema";
int fd = mkstemp(template);
if (fd == -1) die(...);
FILE *f = fdopen(fd, "w");
```

## Issue 2: Potential Global State in Schema Parsing

### The Problem

When running tests in fully parallel mode (default), we observe:

```
FATAL ERROR: OneFile schema error; multiple list types for linetype definition 1
```

This suggests there may be global state in the schema parsing or file opening code that's not thread-safe.

### Current Status

- Tests pass with `--test-threads=1` ✅
- Tests may fail with unrestricted parallelism ❌

### Investigation Needed

Need to check if any of these C functions use global state:
- `oneFileOpenRead()`
- `oneSchemaCreateFromFile()`
- Schema parsing routines

## Workaround for Users

### Running Tests

Always use single-threaded test execution:

```bash
cargo test -- --test-threads=1
```

### Production Use

For production code:
- **Single-threaded**: No issues
- **Multi-threaded**:
  - `OneSchema::from_text()` is safe (protected by mutex)
  - `OneSchema::from_file()` may need external synchronization
  - `OneFile::open_read()` may need external synchronization if opening files concurrently

## Implementation Status

| Issue | Status | Solution |
|-------|--------|----------|
| `from_text()` race condition | ✅ Fixed | Global mutex in Rust wrapper |
| Schema parsing thread safety | ⚠️ Workaround | Use `--test-threads=1` |
| General thread safety audit | ❌ Needed | Comprehensive C library review required |

## Recommendations

### For Rust Users

1. Run tests with `--test-threads=1`
2. If using multiple threads in production, wrap file operations in application-level synchronization
3. Consider creating all schemas at startup before spawning threads

### For C Library Developers

1. **Immediate**: Fix the `oneSchemaCreateFromText()` temp file race condition
2. **Important**: Audit all global and static variables for thread safety
3. **Consider**: Adding an initialization function that sets thread-safety mode
4. **Document**: Clearly state thread-safety guarantees in API documentation

## Testing

To reproduce the issues:

```bash
# This will fail due to race condition (before mutex fix):
cargo test test_schema_from_text

# This may fail due to other threading issues:
cargo test

# This works:
cargo test -- --test-threads=1
```

## References

- ONEcode source: `ONEcode/ONElib.c`
- Rust wrapper: `src/schema.rs`
- Test suite: `tests/basic_tests.rs`
