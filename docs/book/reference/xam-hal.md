# xam HAL

`xam` is the bare-metal HAL (abstract-machine) that kernels link
against. It exposes a minimal set of primitives that xemu knows how
to service.

## Layout

```
xam/
├── include/        HAL headers (C + Rust bindings)
├── src/            implementations (arch-agnostic + riscv-specific)
└── scripts/        build_c.mk, link.ld, cross-target cargo support
```

## API

### Console

```c
void _putch(char ch);       // write one byte to UART TX
```

Used by `xlib`'s `stdio.c` to back `printf`.

### Time

```c
uint64_t mtime(void);               // read ACLINT mtime
void     set_mtimecmp(uint64_t t);  // set MTIMECMP for this hart
uint64_t uptime(void);              // microseconds since boot
```

`uptime()` is derived from `mtime()` divided by 10 (10 MHz clock).

### Trap entry

```rust
pub struct TrapFrame {
    pub regs: [usize; 32],
    pub sstatus: usize,
    pub sepc: usize,
    pub scause: usize,
    pub stval: usize,
}

pub fn init_trap(handler: fn(&mut TrapFrame));
```

Guest sets the handler once at boot; xemu's trap dispatch lands on
it with a populated frame.

### Main-args

```rust
extern "C" {
    static mainargs: *const u8;  // compile-time strings
}
```

Useful for passing test identifiers into a single kernel binary.

### Linker symbols

```
_heap_start      — start of the heap (end of .bss)
_heap_end        — end of the heap (derived from RAM size)
```

### MMIO constants

Device addresses match [Device memory map](./memory-map.md).

## Building a kernel with xam

```bash
cd xkernels/tests/your-kernel
make run
```

The `xam/scripts/build_c.mk` and `build_rs.mk` wrappers handle the
cross-compilation and link script automatically. No target-specific
flags needed in your kernel's Makefile.
