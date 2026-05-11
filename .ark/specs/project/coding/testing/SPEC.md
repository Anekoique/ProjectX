# Testing Guidelines

> Language-agnostic testing rules. Rust-specific assertion policy (`assert!` vs `debug_assert!`) lives in `coding/rust/defensive-programming.md`.

## R1 — Every bug fix is accompanied by a regression test

**Applies to:** `**/*`
**Evidence:** VERIFY

When a bug is fixed, a test that would have caught the bug ships with the fix. Cite the issue number in a comment so future readers can recover the original context.

```rust
// Bad — fix lands without a test
pub fn parse(buf: &[u8]) -> Result<Header> {
    if buf.len() < HEADER_LEN { return Err(Error::Truncated); }
    ...
}

// Good — fix and regression test together
// in tests/parse_header.rs:
#[test]
fn rejects_truncated_header() {
    // Regression for issue #142: previous version panicked on
    // buffers shorter than HEADER_LEN.
    let buf = [0u8; HEADER_LEN - 1];
    assert!(matches!(parse(&buf), Err(Error::Truncated)));
}
```

## R2 — Tests validate user-visible behavior, not internal representation

**Applies to:** `**/tests/**`, `#[ktest]` and `#[test]` functions
**Evidence:** VERIFY

Test through public APIs rather than exposing internal constants in test code. Name tests after the behavior or specification concept being verified, not after internal implementation details. Internal-name coupling makes refactors painful for no gain.

```rust
// Bad — couples test to internal field name
#[test]
fn test_inner_buffer_capacity() {
    let s = MyStream::new();
    assert_eq!(s.inner_buf.capacity(), 4096);
}

// Good — tests observable behavior
#[test]
fn writes_up_to_capacity_without_blocking() {
    let s = MyStream::new();
    let written = s.write_nonblocking(&[0u8; 4096]).unwrap();
    assert_eq!(written, 4096);
}
```

## R3 — Tests use assertion macros, not stdout inspection

**Applies to:** `**/tests/**`, `#[ktest]` and `#[test]` functions
**Evidence:** VERIFY

Use the framework's assertion helpers — `assert!`, `assert_eq!`, `assert_matches!`. Printing values for manual inspection is not testing; assertions provide clear failure messages and make tests self-checking.

```rust
// Bad — manual inspection
#[test]
fn check_parse() {
    let h = parse(&buf).unwrap();
    println!("got {:?}", h);
}

// Good
#[test]
fn parse_returns_expected_header() {
    let h = parse(&buf).unwrap();
    assert_eq!(h.version, 2);
    assert_eq!(h.flags, Flags::DEFAULT);
}
```

## R4 — Tests release every resource they acquire

**Applies to:** `**/tests/**`
**Evidence:** VERIFY

Close file descriptors, unlink temporary files, `waitpid` on child processes. Leftover resources cause flaky failures in subsequent tests. In Rust, prefer RAII guards (`tempfile::TempDir`) so cleanup is automatic.

```c
// Bad — leaks fd and tmp file on test failure
int fd = open("/tmp/test_file", O_CREAT | O_RDWR, 0644);
do_test(fd);

// Good
int fd = open("/tmp/test_file", O_CREAT | O_RDWR, 0644);
do_test(fd);
close(fd);
unlink("/tmp/test_file");
```
