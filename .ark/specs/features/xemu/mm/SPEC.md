[**Goals**]

- G-1: Translate virtual to physical addresses for RV32 (Sv32) and RV64 (Sv39 / Sv48 / Sv57) via descriptor-driven page walks.
- G-2: Cache page-walk results in a 64-entry direct-mapped, ASID-tagged TLB with `sfence.vma` flush semantics.
- G-3: Gate every physical access through a 16-entry PMP table (TOR / NA4 / NAPOT, lock semantics, M-mode fast path).
- G-4: Route physical accesses to RAM or MMIO devices through a single `Bus`, owned inline by `CPU` (no `Arc<Mutex<Bus>>`).
- G-5: Keep MMU / PMP / Bus orthogonal — each layer knows only its own address space and rules.

[**Non-goals**]

- NG-1: No software-managed TLB shootdown — single-hart cooperative emulator inside one OS thread.
- NG-2: No A/D-bit update under `senvcfg.ADUE` (hardware A/D update only).
- NG-3: No DMA-coherent caches; MMIO devices observe writes immediately.

[**Architecture**]

```
xemu/xcore/src/arch/riscv/cpu/
├── mm.rs               MemOp, SvMode, SvConfig, Satp, Pte; impl RVCore { access_bus, checked_read, checked_write, translate, fetch }
└── mm/
    ├── mmu.rs          Mmu { tlb, sv: Option<&'static SvConfig>, satp_ppn, asid, sum, mxr } + translate / update_satp / update_mstatus
    ├── pmp.rs          Pmp [16] + check + update_cfg / update_addr (lock semantics + M-mode fast path)
    └── tlb.rs          Tlb [64] direct-mapped, ASID-tagged + flush(vpn?, asid?)

xemu/xcore/src/device/
├── mod.rs              Device trait, IrqState, DeviceEntry book-keeping
├── bus.rs              Bus { ram, devices, num_harts, reservations } + DmaCtx + LeBytes
└── ram.rs              Ram { base, data } — RAM-backed Device
```

Access path mirrors RV pipeline shape: `vaddr → alignment → MMU translate → paddr → PMP check → Bus access`. Page walks themselves call back into `Bus::read` for PTE reads (PMP-checked).

[**Data Structure**]

```rust
pub enum MemOp { Fetch, Load, Store }

pub enum SvMode { Bare, #[cfg(isa32)] Sv32, #[cfg(isa64)] Sv39, #[cfg(isa64)] Sv48, #[cfg(isa64)] Sv57 }

pub struct SvConfig {
    pub levels: usize, pub pte_size: usize, pub vpn_bits: usize,
    pub va_bits: usize, pub ppn_bits: usize,
}

pub struct Mmu { /* tlb + cached satp_ppn / asid / sv_config / sum / mxr */ }
pub struct Pmp { /* [PmpCfg; 16] + [Word; 16] + has_locked fast-path flag */ }

pub trait Device: Send {
    fn name(&self) -> &str;
    fn read (&mut self, offset: usize, size: usize)              -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, val: Word)   -> XResult;
    fn tick (&mut self)            { /* default no-op */ }
    fn reset(&mut self)            { /* default no-op */ }
    fn mtime(&self) -> Option<u64> { None }
}

pub struct Bus {
    ram:          Ram,
    devices:      Vec<DeviceEntry>,
    num_harts:    usize,
    reservations: Vec<Option<usize>>,
}

pub struct DmaCtx<'a> { /* bus-mediated guest-memory accessor for VirtIO */ }
pub trait LeBytes: Sized {
    fn from_le_bytes_slice(buf: &[u8]) -> Self;
    fn to_le_bytes_vec(self) -> Vec<u8>;
}
```

[**API Surface**]

```rust
impl Mmu {
    pub fn new() -> Self;
    pub fn update_satp    (&mut self, raw: Word);
    pub fn update_mstatus (&mut self, sum: bool, mxr: bool);
    pub fn translate(
        &mut self, hart: HartId, vaddr: VirtAddr, op: MemOp,
        priv_mode: PrivilegeMode, pmp: &Pmp, bus: &mut Bus,
    ) -> XResult<usize>;
}

impl Pmp {
    pub fn new() -> Self;
    pub fn check(&self, paddr: usize, size: usize, op: MemOp, priv_mode: PrivilegeMode) -> XResult;
    pub fn update_cfg (&mut self, index: usize, cfg: u8);
    pub fn update_addr(&mut self, index: usize, addr: usize);
    pub fn get_cfg    (&self, index: usize) -> u8;
    pub fn get_addr   (&self, index: usize) -> usize;
}

impl Bus {
    pub fn new(ram_base: usize, ram_size: usize, num_harts: usize) -> Self;
    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize, dev: Box<dyn Device>) -> usize;
    pub fn replace_device(&mut self, name: &str, dev: Box<dyn Device>);
    pub fn read (&mut self, addr: usize, size: usize)             -> XResult<Word>;
    pub fn store(&mut self, hart: HartId, addr: usize, size: usize, val: Word) -> XResult;
    pub fn tick (&mut self);
    pub fn mtime(&self) -> u64;
    pub fn num_harts(&self) -> usize;
    pub fn reserve(&mut self, hart: HartId, addr: usize);
    pub fn reservation(&self, hart: HartId) -> Option<usize>;
    pub fn clear_reservation (&mut self, hart: HartId);
    pub fn clear_reservations(&mut self);
    pub fn reset_devices     (&mut self);
}
```

[**Constraints**]

- C-1: `Bus` knows only physical addresses + device regions; it never sees virtual addresses, privilege, traps, or PMP — `xemu/xcore/src/device/bus.rs`.
- C-2: `Mmu` knows page tables + TLB + PTE bits + SUM / MXR; it never assigns trap codes — `to_trap` in `RVCore` maps `XError` → `PendingTrap` — `xemu/xcore/src/arch/riscv/cpu/mm.rs:246`.
- C-3: `Pmp` knows physical permissions + privilege; it never touches virtual addresses or page tables — `xemu/xcore/src/arch/riscv/cpu/mm/pmp.rs`.
- C-4: Every physical access goes through `Pmp::check` before reaching `Bus::read` / `Bus::store` — `xemu/xcore/src/arch/riscv/cpu/mm.rs:268`.
- C-5: Page-walk PTE reads are themselves PMP-checked — `Mmu::translate` receives `&Pmp` — `xemu/xcore/src/arch/riscv/cpu/mm/mmu.rs:60`.
- C-6: TLB lookup is ASID-tagged; `sfence.vma` flushes by (vpn?, asid?) — `xemu/xcore/src/arch/riscv/cpu/mm/tlb.rs:80`.
- C-7: `satp` writes refresh `Mmu`'s cached `SvConfig` and trigger TLB invalidation on ASID change — `xemu/xcore/src/arch/riscv/cpu/mm/mmu.rs:43`.
- C-8: The `Device` trait has five methods (`name`, `read`, `write`, default `tick`, default `reset`, default `mtime`); device-lifecycle dispatch lives in `Bus` — `xemu/xcore/src/device/mod.rs:28`.
- C-9: Cross-page accesses are decomposed by the caller (instruction handler), never by `Mmu` or `Bus`.
- C-10: `Bus` is owned inline by `CPU` — no `Arc<Mutex<Bus>>` on the hot path — `xemu/xcore/src/cpu/mod.rs`; enforced by `scripts/ci/verify_no_mutex.sh`.
- C-11: PMP has a M-mode fast path: when no entry has the `L` (lock) bit set and `priv_mode == M`, the 16-entry scan is skipped — `xemu/xcore/src/arch/riscv/cpu/mm/pmp.rs:171`.
- C-12: `DmaCtx` is the only guest-memory accessor for non-CPU paths (VirtIO DMA); no device implementation grabs `&mut Bus` directly — `xemu/xcore/src/device/bus.rs:372`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xemu/xcore/src/arch/riscv/cpu/mm*` and `xemu/xcore/src/device/`. Absorbs the legacy `memOpt` follow-up (single-lock hot path, cached MMU config, PMP fast-path, typed RAM access). Pre-port running-notes preserved at `.ark/tasks/archive/legacy/mm/SPEC_LEGACY.md` and `.ark/tasks/archive/legacy/mem-opt/SPEC_LEGACY.md`.
