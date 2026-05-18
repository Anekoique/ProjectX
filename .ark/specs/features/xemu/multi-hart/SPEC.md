[**Goals**]

- G-1: Generalise `CPU` to own `Vec<Core>` so an emulated machine can carry N harts (`1..=16`).
- G-2: Pass per-hart identity through every CPU API via `HartId(pub u32)` — including bus reservations, ACLINT MSIP / MTIMECMP, and PLIC contexts.
- G-3: Schedule harts cooperatively via round-robin in `CPU::step`; one hart sees a `&mut Bus` borrow per step.
- G-4: Keep the `Core` backend behind the `CoreOps` trait so generic `CPU<C: CoreOps>` works for any architecture.

[**Non-goals**]

- NG-1: No OS-thread per hart — N harts run on one host OS thread; true SMP is a separate phase (RFC).
- NG-2: No hart hot-plug — `num_harts` is fixed at `CPU::new` time.
- NG-3: No per-hart configurable ISA — every hart shares the build's `cfg(isa32)` / `cfg(isa64)` setting.

[**Architecture**]

```
xemu/xcore/src/cpu/
├── core.rs             HartId(u32) + BootMode + trait CoreOps
└── mod.rs              CPU<Core> { cores, bus, current, state, halt_pc, halt_ret, boot_config, boot_layout, uart_line }
                        impl CPU { new, boot, reset, step, run } + advance_current()

xemu/xcore/src/arch/riscv/cpu.rs
└── RVCore impl CoreOps { id, reset, step, halted, ... }
```

`step` destructures `cores` and `bus` as disjoint fields, hands `&mut Bus` to `cores[current]`, advances `current = (current + 1) % cores.len()`. Per-hart state lives in `Core`; shared state (Bus, reservations) lives in `Bus`.

[**Data Structure**]

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HartId(pub u32);

pub enum BootMode { Direct, Firmware }

pub trait CoreOps {
    fn id(&self) -> HartId;
    fn reset(&mut self) -> XResult;
    fn step (&mut self, bus: &mut Bus) -> XResult;
    fn halted(&self) -> bool;
}

#[allow(clippy::upper_case_acronyms)]
pub struct CPU<Core: CoreOps> {
    cores:        Vec<Core>,
    bus:          Bus,                       // owned inline (M-001)
    current:      usize,
    state:        State,
    halt_pc:      VirtAddr,
    halt_ret:     Word,
    boot_config:  BootConfig,
    boot_layout:  BootLayout,
    uart_line:    Option<IrqLine>,
}
```

[**API Surface**]

```rust
impl HartId {
    pub fn as_usize(self) -> usize;
}

impl<C: CoreOps + DebugOps> CPU<C> {
    pub fn new(cores: Vec<C>, bus: Bus, layout: BootLayout) -> Self;
    pub fn bus(&self)         -> &Bus;
    pub fn bus_mut(&mut self) -> &mut Bus;
    pub fn boot (&mut self, cfg: BootConfig) -> XResult;
    pub fn reset(&mut self)                  -> XResult;
    pub fn run  (&mut self, max_steps: u64)  -> XResult;
    pub fn step (&mut self)                  -> XResult;        // one hart, then advance
    pub fn uart_line(&self) -> Option<IrqLine>;
    pub fn is_exit_normal(&self) -> bool;
}

pub static XCPU: OnceLock<Mutex<CPU<Core>>>;
pub fn with_xcpu<R>(f: impl FnOnce(&mut CPU<Core>) -> R) -> R;
```

[**Constraints**]

- C-1: `cores.len() == bus.num_harts()` — checked at `CPU::new` — `xemu/xcore/src/cpu/mod.rs:129`.
- C-2: `num_harts` is in `1..=16` — `xemu/xcore/src/device/bus.rs:147`.
- C-3: `CPU::step` hands exactly one `&mut Bus` borrow to one hart per call — the borrow checker is the exclusion primitive for the cooperative scheduler — `xemu/xcore/src/cpu/mod.rs:241`.
- C-4: `current` advances on every step (halted or not); fairness is round-robin — `xemu/xcore/src/cpu/mod.rs:290`.
- C-5: Per-hart state is owned by `Core`; cross-hart state (LR/SC reservations, MTIMECMP, MSIP, PLIC contexts) is owned by `Bus` or its child devices.
- C-6: The outer `Mutex` on `XCPU` guards the lifecycle handle (xdb / difftest / monitor coordination), not per-instruction access — `xemu/xcore/src/cpu/mod.rs:67`.
- C-7: `CPU<Core>` stays under a 4 KiB size budget — enforced by a `const _` assert in `xemu/xcore/src/cpu/mod.rs:121`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xemu/xcore/src/cpu/`. Pre-port running notes preserved at `.ark/tasks/archive/legacy/multi-hart/`.
