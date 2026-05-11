# Functions and Methods

> Rules for function shape, nesting depth, and parameter design. The goal is functions a reviewer can hold in their head — particularly when the function is part of the unsafe TCB.

## R1 — Nesting depth stays at three or fewer levels

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Each nesting level multiplies reader cognitive load. Flatten with early returns and guard clauses, `let...else`, the `?` operator, `continue` to skip iterations, or extracting nested bodies into helpers. The expected path is the first visible path; error and edge cases handled and dismissed early.

```rust
// Bad — main path buried under nested error handling
pub(crate) fn init() {
    if let Some(framebuffer_arg) = boot_info().framebuffer_arg {
        if let Some(addr) = framebuffer_arg.address {
            if let Ok(mapping) = map_framebuffer(addr) {
                set_framebuffer(mapping);
            }
        }
    }
}

// Good — early returns let the main path read top-to-bottom
pub(crate) fn init() {
    let Some(framebuffer_arg) = boot_info().framebuffer_arg else {
        warn!("framebuffer not found");
        return;
    };
    let Some(addr) = framebuffer_arg.address else { return; };
    let mapping = match map_framebuffer(addr) {
        Ok(m) => m,
        Err(e) => { warn!("framebuffer mapping failed: {e}"); return; }
    };
    set_framebuffer(mapping);
}
```

## R2 — A function does one thing at one level of abstraction

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

If a helper can be extracted with a name that does not merely restate its implementation, the original function was doing more than one thing. Do not mix levels: a syscall handler reads like a specification; byte-level manipulation belongs in a helper.

```rust
// Bad — high-level dispatch mixed with byte-level parsing
pub fn sys_connect(sockfd: i32, addr: Vaddr, len: u32) -> Result<()> {
    let fd_table = current_process().fd_table().lock();
    let file = fd_table.get(sockfd).ok_or(Errno::EBADF)?;
    let socket = file.downcast_ref::<Socket>().ok_or(Errno::ENOTSOCK)?;
    let bytes = read_bytes_from_user(addr, len as usize)?;
    let family = u16::from_ne_bytes([bytes[0], bytes[1]]);
    // ... 30 more lines of byte parsing ...
}

// Good — each function operates at one level
pub fn sys_connect(sockfd: i32, addr: Vaddr, len: u32) -> Result<()> {
    let socket = get_socket(sockfd)?;
    let remote_addr = parse_socket_addr(addr, len)?;
    socket.connect(remote_addr)
}
```

## R3 — Functions do not take boolean parameters; use enums or split the function

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

A `bool` parameter that selects between two behaviors signals the function does two things. The call site `read(buf, true)` is uninterpretable without checking the signature; `read_blocking(buf)` is self-documenting.

```rust
// Bad — call sites become unreadable
fn read(&self, buf: &mut [u8], blocking: bool) -> Result<usize> { ... }

// Good — two functions
fn read(&self, buf: &mut [u8]) -> Result<usize> { ... }
fn read_nonblocking(&self, buf: &mut [u8]) -> Result<usize> { ... }

// Good — typed enum when the choice is data
enum ReadMode { Blocking, NonBlocking }
fn read(&self, buf: &mut [u8], mode: ReadMode) -> Result<usize> { ... }
```
