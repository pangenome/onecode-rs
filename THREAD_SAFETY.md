# Thread Safety Issues in ONEcode C Library

## Overview

The ONEcode C library has several thread safety issues that affect concurrent usage within a single process. This document describes these issues and the workarounds implemented in the Rust wrapper.

## Issue 1: Global Error String Buffer

### The Problem

The ONEcode C library uses a **global static buffer** for error messages that is not thread-safe.

### Root Cause

From `ONElib.c` line 94:

```c
static char errorString[1024];

char *oneErrorString(void) { return errorString; }
```

Any function that fails writes to this global buffer using `snprintf`:

```c
#define OPEN_ERROR1(x) \
    { snprintf(errorString, 1024, "ONEcode file open error %s: %s\n", localPath, x); \
      fclose(f); if (localPath != path) free(localPath); return NULL; }
```

**The bug**: When multiple threads call functions that might fail:

1. Thread A calls `oneFileOpenRead("bad1.seq", ...)` which fails
2. Thread B calls `oneFileOpenRead("bad2.seq", ...)` which fails and **overwrites errorString**
3. Thread A calls `oneErrorString()` and gets Thread B's error message!

Or worse, if Thread A is reading errorString while Thread B is writing, we get **undefined behavior** (data race, potential crash).

### Symptoms

- Wrong error messages reported
- Corrupted error strings
- Potential crashes when reading error strings concurrently

### Rust Wrapper Solution

We added a global mutex in `src/file.rs`:

```rust
static ERROR_STRING_LOCK: Mutex<()> = Mutex::new(());

pub fn open_read(...) -> Result<Self> {
    // Lock BEFORE calling C function (which may write to errorString)
    let _guard = ERROR_STRING_LOCK.lock().unwrap();

    let ptr = ffi::oneFileOpenRead(...);
    if ptr.is_null() {
        // Still holding lock while reading error string
        let err_str = ffi::oneErrorString();
        let err_msg = CStr::from_ptr(err_str).to_string_lossy().into_owned();
        return Err(OneError::OpenFailed(err_msg));
    }
    // Lock released here
    Ok(OneFile { ptr, is_owned: true })
}
```

**Critical**: The lock must be held during BOTH:
1. The C function call (which may write to errorString)
2. Reading the errorString

This ensures atomicity of the fail-and-report-error operation.

### Recommended Fix for C Library

Use thread-local storage or return error codes:

```c
// Option 1: Thread-local storage (C11)
_Thread_local char errorString[1024];

// Option 2: Return error codes and error messages together
typedef struct {
    int code;
    char message[1024];
} OneError;

OneFile *oneFileOpenRead(..., OneError *err);
```

## Issue 2: `oneSchemaCreateFromText()` Race Condition

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

## Issue 3: Potential Global State in Schema Parsing

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
| Global error string buffer | ✅ Fixed | Mutex around C calls + error reads |
| `from_text()` temp file race | ✅ Fixed | Mutex around `oneSchemaCreateFromText()` |
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
