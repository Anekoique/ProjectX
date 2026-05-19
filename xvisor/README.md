# xvisor

Type-1 RISC-V hypervisor. Runs in HS-mode above OpenSBI fw_jump on the
QEMU `virt` machine; uses the H-extension to host VS-mode guests.

## Boot it

```
cd xvisor && make run
```

Expected: a banner of the form

```
xvisor: hello from HS-mode (hartid=0, dtb=0x<addr>)
```

over the ns16550 UART, followed by clean QEMU shutdown (`make run` returns 0).

The QEMU launch line is:

```
qemu-system-riscv64 -nographic -machine virt -cpu rv64,h=true \
                    -smp 1 -m 256M -bios default -kernel xvisor.elf
```

## Make targets

| target        | does                                                       |
| ------------- | ---------------------------------------------------------- |
| `make`        | alias of `make build`                                      |
| `make build`  | `cargo build --release` for `riscv64gc-unknown-none-elf`   |
| `make run`    | build + launch QEMU on `-machine virt -cpu rv64,h=true`    |
| `make fmt`    | `cargo fmt --all`                                          |
| `make clippy` | `cargo clippy --bins -- -D warnings`                       |
| `make test`   | placeholder — no host-runnable tests yet                   |
| `make clean`  | `cargo clean`                                              |

## Layout

```
xvisor/
├── Cargo.toml      no_std binary; default feature = "platform-qemu"
├── Makefile        build/run/fmt/clippy/test/clean targets
├── linker.ld       BASE = 0x80200000; .bss.stack + .bss layout
├── build.rs        emits `-T linker.ld` link-arg
└── src/
    ├── main.rs            crate root, rust_main, panic handler
    ├── hal/               hardware abstraction layer (cfg_attr-selected backends)
    │   ├── arch/riscv/    naked_asm! _start, PerCpu, CSRs, TrapFrame layout
    │   ├── arch/loongarch (stub)
    │   ├── platform/qemu  ns16550 UART + SiFive-test finisher
    │   └── platform/xemu  (stub)
    ├── mm/                (empty) hyp heap + G-stage builder
    ├── vcpu/              (empty) vCPU register file + run loop
    ├── vm/                (empty) per-guest VM struct
    └── sbi/               (empty) inbound SBI ecall dispatch
```

`mm/`, `vcpu/`, `vm/`, `sbi/`, `hal/arch/loongarch`, `hal/platform/xemu` are
committed as empty modules so the public vocabulary is fixed; their contents
land in later features.

## Operator notes

- **Forgot `-cpu rv64,h=true`.** xvisor cannot detect this from HS-mode (the
  `misa` CSR is M-mode-only). OpenSBI itself flags the miss in its startup
  banner; the xvisor banner won't appear.
- **`-smp 2`.** Secondary harts spin in OpenSBI's HSM until raised. Only
  hart 0 reaches `_start`; the banner still prints exactly once.

## Roadmap

See `docs/XVISOR.md`. This crate currently delivers the boot-and-banner
foundation; trap entry, H-extension setup, G-stage paging, vCPU run-loop,
SBI dispatch, and device emulation follow in subsequent features.
