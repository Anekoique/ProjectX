# `mm` SPEC

> Memory subsystem — Bus, MMU, TLB, PMP, MMIO routing.
>
> **Source:** [`/docs/archived/feat/mm/MEM_PLAN.md`](/docs/archived/feat/mm/MEM_PLAN.md) — pre-workflow design document,
> preserved verbatim as the authoritative spec for this feature.
> The layout does not match `docs/template/SPEC.template`; rewrite
> to the template shape when the feature next sees meaningful
> iteration.

---

# Memory Subsystem Implementation Plan

> Phase 3 of [PROGRESS.md](../PROGRESS.md) — Memory Management
> Bus + MMU + TLB + MMIO routing, Rust-idiomatic, dual RV32/RV64
>
> **Current: v9** (2026-03-25)
>
> Changelog:
> - v9: SvMode descriptor for runtime SV32/39/48/57 switching. Walk parameterized
>   by &SvMode. Pte format-dependent methods take &SvMode. MMU caches Option<&'static SvMode>.
> - v8: MemOp replaces AccessKind. MemFault eliminated — reuse XError::PageFault + BadAddress.
> - v7: MMU caches satp/mstatus config. No AccessCtx struct. translate(vaddr, op, priv).
> - v6: Bus shared via Arc<Mutex<Bus>>. No parameter threading on instruction handlers.
> - v5: RAM/MMIO split in Bus. AccessCtx/MemFault. err2trap pattern. MemAccess deleted.
> - v4: Initial plan — Pte type with PteFlags, page walk, TLB, sfence.vma.
>
> Reference designs: KXemu, Nemu-rust, REMU, arceos, asterinas.

---

## Architecture Overview

```
CPU                                         ┌──────────────────────┐
├── core: RVCore                            │  Bus                 │
│   ├── bus: Arc<Mutex<Bus>> (clone) ──────►│  ├── Ram [0x8000_0000│
│   ├── mmu: Mmu  ──vaddr→paddr            │  ├── UART [Phase 4]  │
│   │   └── tlb: Tlb                        │  └── ...             │
│   ├── pmp: Pmp  ──paddr permission gate   └──────────────────────┘
│   ├── csr, privilege, ...
│   └── step(&mut self)
├── bus: Arc<Mutex<Bus>>  ◄─── shared via Arc clone
└── state, halt_pc, halt_ret

Access path (models real RISC-V pipeline):
  vaddr ─► alignment check ─► MMU translate ─► paddr ─► PMP check ─► bus access
                                   │                        ▲
                                   └── page walk: pte_paddr ┘  (PMP also checks PTE reads)
```

Four-layer responsibility split:

| Layer | Knows about | Does NOT know about |
|-------|------------|---------------------|
| `Bus` | Physical addresses, device regions | Virtual addresses, privilege, traps, PMP |
| `Mmu` | Page tables, TLB, PTE bits, SUM/MXR | Trap codes, PMP (receives `&Pmp` for walks) |
| `Pmp` | Physical address permissions, privilege | Virtual addresses, page tables |
| `RVCore` | Orchestrates: privilege, MPRV, trap mapping | Internal device state |

---

## Design Decisions

### 1. Device trait + Bus

```rust
// xcore/src/device/mod.rs

pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
}
```

Two methods. That's the entire device interface. Matches:
- KXemu `MMIODev`: `read(offset, size, &valid)` + `write(offset, data, size)`
- Nemu-rust `IOMap`: `read(offset, len)` + `write(offset, data, len)`

```rust
// xcore/src/device/bus.rs

struct MmioRegion {
    name: &'static str,
    base: usize,
    size: usize,
    dev: Box<dyn Device>,
}

pub struct Bus {
    ram: Ram,
    ram_base: usize,
    mmio: Vec<MmioRegion>,
}

impl Bus {
    pub fn new(ram_base: usize, ram_size: usize) -> Self {
        Self {
            ram: Ram::new(ram_size),
            ram_base,
            mmio: Vec::new(),
        }
    }

    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize, dev: Box<dyn Device>) {
        assert!(size > 0, "region size must be non-zero");
        assert!(base.checked_add(size).is_some(), "region overflows address space");
        // Check overlap with RAM
        let ram_end = self.ram_base + self.ram.len();
        let no_ram_overlap = base + size <= self.ram_base || ram_end <= base;
        assert!(no_ram_overlap, "MMIO '{}' [{:#x}..{:#x}) overlaps RAM", name, base, base + size);
        // Check overlap with other MMIO regions
        for r in &self.mmio {
            let no_overlap = base + size <= r.base || r.base + r.size <= base;
            assert!(no_overlap, "MMIO '{}' [{:#x}..{:#x}) overlaps '{}'", name, base, base + size, r.name);
        }
        self.mmio.push(MmioRegion { name, base, size, dev });
    }

    /// Read from any physical address (RAM or MMIO).
    pub fn read(&mut self, addr: usize, size: usize) -> XResult<Word> {
        // Fast path: RAM (static dispatch, no vtable)
        if addr >= self.ram_base && addr + size <= self.ram_base + self.ram.len() {
            return self.ram.read(addr - self.ram_base, size);
        }
        // Slow path: MMIO (dynamic dispatch)
        let (dev, offset) = self.find_mmio(addr, size)?;
        dev.read(offset, size)
    }

    /// Write to any physical address (RAM or MMIO).
    pub fn write(&mut self, addr: usize, size: usize, value: Word) -> XResult {
        if addr >= self.ram_base && addr + size <= self.ram_base + self.ram.len() {
            return self.ram.write(addr - self.ram_base, size, value);
        }
        let (dev, offset) = self.find_mmio(addr, size)?;
        dev.write(offset, size, value)
    }

    /// Read from RAM only. Used by page table walk and image loading.
    /// Returns `Err(BadAddress)` if the address hits MMIO or is unmapped.
    pub fn read_ram(&self, addr: usize, size: usize) -> XResult<Word> {
        if addr >= self.ram_base && addr + size <= self.ram_base + self.ram.len() {
            return self.ram.read(addr - self.ram_base, size);
        }
        Err(XError::BadAddress)
    }

    /// Bulk load bytes directly into RAM (for image/ELF loading).
    pub fn load_ram(&mut self, addr: usize, data: &[u8]) -> XResult {
        let offset = addr.checked_sub(self.ram_base).ok_or(XError::BadAddress)?;
        if offset + data.len() > self.ram.len() {
            return Err(XError::BadAddress);
        }
        self.ram.load(offset, data)
    }

    fn find_mmio(&mut self, addr: usize, size: usize) -> XResult<(&mut dyn Device, usize)> {
        for r in &mut self.mmio {
            if addr >= r.base && addr + size <= r.base + r.size {
                return Ok((r.dev.as_mut(), addr - r.base));
            }
        }
        Err(XError::BadAddress)
    }
}
```

**RAM/MMIO split**: RAM is stored directly (no `Box<dyn Device>`, no vtable overhead).
MMIO regions use dynamic dispatch. This gives static dispatch for 99.9% of accesses
while keeping the MMIO path flexible for Phase 4 devices.

**Why split**: Page table walks, image loading, and future AMO paths must only hit RAM.
`read_ram()` / `load_ram()` enforce this structurally — a misconfigured `satp` pointing
at a UART register returns `BadAddress`, not a spurious device read.

**MMIO linear scan**: ~5 devices, O(1) in practice. Same as KXemu, REMU, Nemu-rust.

### 2. Ram

```rust
// xcore/src/device/ram.rs

pub struct Ram {
    data: Vec<u8>,
}

impl Ram {
    pub fn new(size: usize) -> Self {
        Self { data: vec![0; size] }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Raw little-endian read. No alignment checks.
    pub fn read(&self, offset: usize, size: usize) -> XResult<Word> {
        let mut buf = [0u8; std::mem::size_of::<Word>()];
        buf[..size].copy_from_slice(&self.data[offset..offset + size]);
        Ok(Word::from_le_bytes(buf))
    }

    /// Raw little-endian write. No alignment checks.
    pub fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult {
        let bytes = value.to_le_bytes();
        self.data[offset..offset + size].copy_from_slice(&bytes[..size]);
        Ok(())
    }

    /// Bulk byte copy for image loading.
    pub fn load(&mut self, offset: usize, data: &[u8]) -> XResult {
        self.data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }
}
```

**Ram is NOT a `Device`.** It is owned directly by `Bus`, accessed without vtable
dispatch. The `Device` trait is reserved for MMIO peripherals (UART, CLINT, etc.)
added in Phase 4.

**No alignment checks in Ram.** Ram does raw byte access. Architectural alignment rules
are enforced by RVCore before the bus call (see §4). This prevents the "bus returns
`AddrNotAligned`, caller guesses which trap" problem.

**`Ram::read` takes `&self`, not `&mut self`.** RAM reads have no side effects.
This allows `Bus::read_ram(&self, ...)` for page table walks without requiring
`&mut Bus`, which is important when the MMU borrows the bus during translation.

### 3. Bus ownership: `Arc<Mutex<Bus>>`, shared between CPU and cores

**Current** (xemu today):
```rust
// Global memory — mutex on every access
pub static MEMORY: LazyLock<Mutex<Memory>> = ...;
macro_rules! with_mem { ... MEMORY.lock()... }

// CoreOps::step takes no bus parameter
pub trait CoreOps {
    fn step(&mut self) -> XResult;
}

// RVCore calls with_mem! directly
fn fetch(&self) -> XResult<u32> {
    let word = with_mem!(fetch_u32(self.virt_to_phys(self.pc), 4))?;
    ...
}
```

**After** (this plan):
```rust
use std::sync::{Arc, Mutex};

// CPU owns the Arc, clones to core
pub struct CPU<Core: CoreOps> {
    core: Core,
    bus: Arc<Mutex<Bus>>,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
}

// CoreOps::step — no bus parameter
pub trait CoreOps {
    fn step(&mut self) -> XResult;
    ...
}

// RVCore holds its own Arc clone
pub struct RVCore {
    gpr: [Word; 32],
    pc: VirtAddr,
    bus: Arc<Mutex<Bus>>,  // clone — same Bus as CPU
    ...
}

impl RVCore {
    fn bus(&self) -> std::sync::MutexGuard<'_, Bus> {
        self.bus.lock().expect("bus lock poisoned")
    }
}

// CPU creates the shared bus, clones to core
impl<Core: CoreOps> CPU<Core> {
    pub fn step(&mut self) -> XResult {
        self.core.step()?;  // no bus parameter
        ...
    }
}

// RVCore accesses bus via self.bus() — no parameter threading
fn fetch(&mut self) -> XResult<u32> {
    let paddr = self.translate_fetch(self.pc)?;
    let word = self.bus().read(paddr, 4)?;
    ...
}
```

**Why `Arc<Mutex<Bus>>`**:
- No `&mut Bus` parameter threading through dispatch + ~70 instruction handlers
- Multi-core ready: `CPU { cores: Vec<Core>, bus: Arc<Mutex<Bus>> }` — each core gets a clone
- Bus has the same lifetime as CPU — Arc ensures this naturally
- Single-core: mutex is uncontested, ~20ns overhead per access (acceptable)
- Instruction handlers access bus via `self.bus()` like any other field

**What gets removed**: `static MEMORY`, `with_mem!` macro, `MemOps` trait.
**What stays simple**: `CoreOps::step(&mut self)` — no bus parameter.
Instruction handler signatures unchanged from original — only memory helpers
(`fetch`, `load_op`, `store_op`, `amo_w`, `lr`, `sc`) call `self.bus()`.

### 4. MMU: cached config, minimal translate interface

**Types**:

```rust
// xcore/src/cpu/riscv/mmu.rs

/// Memory operation type — determines PTE permission bit and fault cause.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemOp { Fetch, Load, Store, Amo }

pub struct Mmu {
    tlb: Tlb,
    // Cached from satp — updated on CSR write
    sv: Option<&'static SvMode>,  // None = Bare mode (identity mapping)
    ppn: usize,                    // page table root PPN
    asid: u16,
    // Cached from mstatus — updated on CSR write
    sum: bool,
    mxr: bool,
}
```

**No `MemFault` enum.** Translation failures use existing `XError` variants:
- `XError::PageFault` (new) — PTE invalid, permission denied, A/D, canonical
- `XError::BadAddress` (existing) — PTE address unmapped, PMP denied

RVCore maps these to RISC-V traps based on `MemOp`:
- `PageFault` + `Fetch` → `InstructionPageFault`
- `BadAddress` + `Load` → `LoadAccessFault`
- etc.

The `translate` call passes only what varies per access:

```rust
impl Mmu {
    /// Update cached satp fields. Called on satp CSR write.
    pub fn update_satp(&mut self, satp: Word) {
        self.sv = match satp_mode(satp) {
            0 => None,                    // Bare
            1 => Some(&SV32),             // RV32 only
            8 => Some(&SV39),
            9 => Some(&SV48),
            10 => Some(&SV57),
            _ => None,                    // reserved → Bare
        };
        self.ppn = satp_ppn(satp);
        self.asid = satp_asid(satp);
        self.tlb.flush(None, None);
    }

    /// Update cached mstatus fields. Called on mstatus CSR write.
    pub fn update_mstatus(&mut self, sum: bool, mxr: bool) {
        self.sum = sum;
        self.mxr = mxr;
    }

    /// Translate virtual → physical. Returns XResult for uniform error handling.
    pub fn translate(
        &mut self,
        vaddr: VirtAddr,
        op: MemOp,
        priv_mode: PrivilegeMode,
        pmp: &Pmp,
        bus: &MutexGuard<'_, Bus>,
    ) -> XResult<usize> {
        // Bare mode or M-mode: identity mapping
        let Some(sv) = self.sv else {
            return Ok(vaddr.as_usize());
        };
        if priv_mode == PrivilegeMode::Machine {
            return Ok(vaddr.as_usize());
        }

        // TLB lookup
        let vpn = vaddr_vpn(vaddr);
        if let Some(entry) = self.tlb.lookup(vpn, self.asid) {
            if entry.check_perm(op, priv_mode, self.sum, self.mxr) {
                return Ok(entry.translate(vaddr, sv));
            }
        }

        // TLB miss → page walk
        let entry = self.page_walk(vaddr, op, priv_mode, sv, pmp, bus)?;
        let paddr = entry.translate(vaddr, sv);
        self.tlb.insert(entry);
        Ok(paddr)
    }
}
```

**Design rationale** (follows KXemu's `vaddr_translate_core` pattern):
- KXemu caches satp mode as a function pointer, updated on satp write
- We cache mode/ppn/asid/sum/mxr as fields — same idea, Rust-idiomatic
- `translate` is called ~billions of times; `update_satp`/`update_mstatus` ~rarely
- `priv_mode` still passed per-call because MPRV changes it for data vs fetch

**RVCore orchestrates the full access path** — MMU, PMP, and bus:

```rust
impl RVCore {
    fn effective_priv(&self) -> PrivilegeMode {
        if self.privilege == PrivilegeMode::Machine && self.csr.mstatus().mprv() {
            self.csr.mstatus().mpp()
        } else {
            self.privilege
        }
    }

    /// Full access path: vaddr → MMU translate → PMP check → paddr.
    /// Maps XError::{PageFault, BadAddress} to the correct RISC-V trap.
    fn translate(
        &mut self, vaddr: VirtAddr, op: MemOp, priv_mode: PrivilegeMode,
        bus: &MutexGuard<'_, Bus>,
    ) -> XResult<usize> {
        let paddr = self.mmu
            .translate(vaddr, op, priv_mode, &self.pmp, bus)
            .map_err(|e| self.map_mem_err(e, vaddr, op))?;

        // PMP on final paddr (walk PTEs already PMP-checked inside MMU)
        self.pmp
            .check(paddr, op, self.privilege)  // original privilege, not effective
            .map_err(|e| self.map_mem_err(e, vaddr, op))?;

        Ok(paddr)
    }

    /// Map XError::{PageFault, BadAddress} → RISC-V trap based on MemOp.
    fn map_mem_err(&self, err: XError, vaddr: VirtAddr, op: MemOp) -> XError {
        let tval = vaddr.as_usize() as Word;
        let exc = match err {
            XError::PageFault => match op {
                MemOp::Fetch => Exception::InstructionPageFault,
                MemOp::Load  => Exception::LoadPageFault,
                _            => Exception::StorePageFault,
            },
            XError::BadAddress => match op {
                MemOp::Fetch => Exception::InstructionAccessFault,
                MemOp::Load  => Exception::LoadAccessFault,
                _            => Exception::StoreAccessFault,
            },
            other => return other,  // pass through non-memory errors
        };
        XError::Trap(PendingTrap { cause: TrapCause::Exception(exc), tval })
    }
}
```

**PMP check uses `self.privilege` (original), not `priv_mode` (effective).**
Per spec §3.7.1: PMP applies to S/U-mode accesses and page-table walks
(effective = S). M-mode bypasses unless Locked.

Then fetch/load/store — `map_mem_err` handles bus errors too:

```rust
impl RVCore {
    fn bus(&self) -> MutexGuard<'_, Bus> {
        self.bus.lock().expect("bus lock poisoned")
    }

    fn fetch(&mut self) -> XResult<u32> {
        if !self.pc.is_aligned(2_usize) {
            return self.trap_exception(Exception::InstructionMisaligned,
                                       self.pc.as_usize() as Word);
        }
        let bus = self.bus();
        let paddr = self.translate(self.pc, MemOp::Fetch, self.privilege, &bus)?;
        let word = bus.read(paddr, 4)
            .map_err(|e| self.map_mem_err(e, self.pc, MemOp::Fetch))?;
        Ok(word_to_u32(word))
    }

    fn load_op<F>(&mut self, rd: RVReg, rs1: RVReg, imm: SWord, size: usize,
                   extend: F) -> XResult
    where F: FnOnce(Word) -> Word {
        let vaddr = self.eff_addr(rs1, imm);
        if !vaddr.is_aligned(size) {
            return self.trap_exception(Exception::LoadMisaligned,
                                       vaddr.as_usize() as Word);
        }
        let bus = self.bus();
        let paddr = self.translate(vaddr, MemOp::Load, self.effective_priv(), &bus)?;
        let value = bus.read(paddr, size)
            .map_err(|e| self.map_mem_err(e, vaddr, MemOp::Load))?;
        self.set_gpr(rd, extend(value))
    }

    fn store_op(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord, size: usize) -> XResult {
        let vaddr = self.eff_addr(rs1, imm);
        if !vaddr.is_aligned(size) {
            return self.trap_exception(Exception::StoreMisaligned,
                                       vaddr.as_usize() as Word);
        }
        let bus = self.bus();
        let paddr = self.translate(vaddr, MemOp::Store, self.effective_priv(), &bus)?;
        let mask = if size >= std::mem::size_of::<Word>() { Word::MAX }
                   else { (1 as Word).wrapping_shl(size as u32 * 8) - 1 };
        bus.write(paddr, size, self.gpr[rs2] & mask)
            .map_err(|e| self.map_mem_err(e, vaddr, MemOp::Store))?;
        self.reservation = None;
        Ok(())
    }
}
```

**Full access path**: alignment → lock bus → MMU translate → PMP check → bus access.
Bus errors (`BadAddress`) also flow through `map_mem_err` — unified error mapping.

### 5. Page table walk

Follows RISC-V spec §4.3.2 Sv39 / §4.3.1 Sv32. Designed for runtime mode
switching (satp changes active scheme) and future SV48/SV57 extensibility.

#### 5a. SvMode descriptor — one struct for all page table formats

```rust
/// Page table format descriptor. One const per Sv scheme.
/// Adding SV48/SV57 is just two more constants — zero code changes to the walk.
struct SvMode {
    levels: usize,     // 2 (SV32) / 3 (SV39) / 4 (SV48) / 5 (SV57)
    pte_size: usize,   // 4 (SV32) / 8 (SV39+)
    vpn_bits: usize,   // 10 (SV32) / 9 (SV39+)
    va_bits: usize,    // 32 / 39 / 48 / 57
}

const SV32: SvMode = SvMode { levels: 2, pte_size: 4, vpn_bits: 10, va_bits: 32 };
const SV39: SvMode = SvMode { levels: 3, pte_size: 8, vpn_bits: 9,  va_bits: 39 };
const SV48: SvMode = SvMode { levels: 4, pte_size: 8, vpn_bits: 9,  va_bits: 48 };
const SV57: SvMode = SvMode { levels: 5, pte_size: 8, vpn_bits: 9,  va_bits: 57 };

const PAGE_SHIFT: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
```

**Design rationale** (compare reference projects):
- KXemu uses C++ templates `<LEVELS, PTESIZE, VPNBITS>` — compile-time instantiation,
  function pointer swapped on satp write. Our `SvMode` struct is the Rust equivalent.
- arceos uses trait `PagingMetaData { const LEVELS, VA_MAX_BITS }` with separate types
  per scheme (`Sv39MetaData`, `Sv48MetaData`). Good for OS page table management, but
  an emulator needs runtime switching — satp changes which mode is active.
- asterinas uses `PagingConstsTrait` with `#[cfg]` feature gates. Compile-time only —
  can't switch modes at runtime.

Our approach: **runtime-parameterized walk via `&SvMode`**, cached in `Mmu` on satp
write. The walk loop is generic over the descriptor. SV48/SV57 require no code
changes — just use the additional constants.

#### 5b. Pte type — pure bitfield accessor

```rust

bitflags::bitflags! {
    struct PteFlags: usize {
        const V = 1 << 0; const R = 1 << 1; const W = 1 << 2;
        const X = 1 << 3; const U = 1 << 4; const G = 1 << 5;
        const A = 1 << 6; const D = 1 << 7;
    }
}

/// Decoded PTE. Pure bit accessor — no policy logic here.
/// Methods that depend on the page table format take `&SvMode`.
#[derive(Clone, Copy)]
struct Pte(usize);

impl Pte {
    fn flags(self) -> PteFlags { PteFlags::from_bits_truncate(self.0) }
    fn is_valid(self) -> bool   { self.flags().contains(PteFlags::V) }
    fn is_leaf(self) -> bool    { self.flags().intersects(PteFlags::R | PteFlags::X) }
    fn is_reserved(self) -> bool {
        self.flags().contains(PteFlags::W) && !self.flags().contains(PteFlags::R)
    }

    fn ppn(self, sv: &SvMode) -> usize {
        (self.0 >> 10) & ((1 << (sv.levels * sv.vpn_bits + 2)) - 1)
    }

    fn superpage_aligned(self, level: usize, sv: &SvMode) -> bool {
        level == 0 || (self.ppn(sv) & ((1 << (level * sv.vpn_bits)) - 1)) == 0
    }

    fn translate(self, vaddr: VirtAddr, level: usize, sv: &SvMode) -> usize {
        if level > 0 {
            let mask = (1 << (level * sv.vpn_bits + PAGE_SHIFT)) - 1;
            (self.ppn(sv) << PAGE_SHIFT) & !mask | (vaddr.as_usize() & mask)
        } else {
            self.ppn(sv) << PAGE_SHIFT | (vaddr.as_usize() & (PAGE_SIZE - 1))
        }
    }
}
```

`Pte` is a pure bitfield — no permission policy. Format-dependent methods
(`ppn`, `superpage_aligned`, `translate`) take `&SvMode` for the active
page table scheme. Permission checking lives in the walk.

#### 5b. The walk — permission check inline

```rust
impl Mmu {
    /// Walk page table, return TlbEntry for caching + translation.
    fn page_walk(
        &self,
        vaddr: VirtAddr,
        op: MemOp,
        priv_mode: PrivilegeMode,
        sv: &SvMode,
        pmp: &Pmp,
        bus: &MutexGuard<'_, Bus>,
    ) -> XResult<TlbEntry> {
        if !is_canonical(vaddr, sv) { return Err(XError::PageFault); }

        let mut base = self.ppn * PAGE_SIZE;

        for level in (0..sv.levels).rev() {
            let pte_addr = base + vpn_index(vaddr, level, sv) * sv.pte_size;

            // PMP on PTE read (effective privilege = S, spec §3.7.1)
            pmp.check(pte_addr, MemOp::Load, PrivilegeMode::Supervisor)?;

            let pte = self.read_pte(pte_addr, sv, bus)?;

            if !pte.is_valid() || pte.is_reserved() {
                return Err(XError::PageFault);
            }

            if pte.is_leaf() {
                if !pte.superpage_aligned(level, sv)
                    || !self.check_leaf_perm(pte, op, priv_mode) {
                    return Err(XError::PageFault);
                }
                return Ok(TlbEntry::from_pte(pte, vaddr, level, sv, self.asid));
            }

            base = pte.ppn(sv) * PAGE_SIZE;
        }

        Err(XError::PageFault)
    }

    fn check_leaf_perm(&self, pte: Pte, op: MemOp, priv_mode: PrivilegeMode) -> bool {
        let f = pte.flags();

        let perm_ok = match op {
            MemOp::Fetch => f.contains(PteFlags::X),
            MemOp::Load  => f.contains(PteFlags::R) || (self.mxr && f.contains(PteFlags::X)),
            _            => f.contains(PteFlags::W),
        };

        let priv_ok = if f.contains(PteFlags::U) {
            priv_mode == PrivilegeMode::User
                || (priv_mode == PrivilegeMode::Supervisor && self.sum)
        } else {
            priv_mode != PrivilegeMode::User
        };

        // Svade: A must be set; D must be set for writes
        let ad_ok = f.contains(PteFlags::A)
            && (!matches!(op, MemOp::Store | MemOp::Amo) || f.contains(PteFlags::D));

        perm_ok && priv_ok && ad_ok
    }

    fn read_pte(&self, addr: usize, sv: &SvMode, bus: &MutexGuard<'_, Bus>) -> XResult<Pte> {
        bus.read_ram(addr, sv.pte_size).map(|w| Pte(w as usize))
    }
}

fn vpn_index(vaddr: VirtAddr, level: usize, sv: &SvMode) -> usize {
    (vaddr.as_usize() >> (PAGE_SHIFT + level * sv.vpn_bits)) & ((1 << sv.vpn_bits) - 1)
}

/// Canonical address check: upper bits must be sign-extension of bit[va_bits-1].
fn is_canonical(vaddr: VirtAddr, sv: &SvMode) -> bool {
    let va = vaddr.as_usize() as isize;
    let shift = usize::BITS as usize - sv.va_bits;
    (va << shift) >> shift == va
}
```

**Key change from v5**: Permission checking is `Mmu::check_leaf_perm()` — a method
on MMU that reads `self.sum`/`self.mxr` directly. No `AccessCtx` struct needed.
`Pte` is a pure bitfield accessor with no policy logic.

Walk uses `self.ppn` (cached) instead of parsing satp each call.
`self.asid` passed to `TlbEntry::from_pte` — also cached.

### 6. A/D bit policy: Svade

**Choice**: Raise page fault when A=0, or when writing with D=0.

| Scenario | A | D | Access | Result |
|----------|---|---|--------|--------|
| First read | 0 | 0 | Load | PageFault (A=0) |
| After OS sets A | 1 | 0 | Load | OK |
| First write | 1 | 0 | Store | PageFault (D=0) |
| After OS sets A+D | 1 | 1 | Store | OK |

**Why Svade**: Simpler (no PTE write-back during walk). Matches QEMU/Spike defaults.
Linux handles A/D faults in its page fault handler.

**If Phase 7 needs hardware A/D**: Change `page_walk` to write back PTEs. The `translate`
signature doesn't change — only internal walk logic changes.

### 7. PMP (Physical Memory Protection)

Separate from MMU. Operates on physical addresses only. Checked at two points:
1. **Final paddr** — in `RVCore::translate()`, after MMU returns paddr
2. **PTE reads during page walk** — in `Mmu::page_walk()`, before each `read_pte`

```rust
// xcore/src/cpu/riscv/pmp.rs

const PMP_COUNT: usize = 16;

/// Address matching mode (pmpcfg.A field).
#[derive(Clone, Copy, PartialEq, Eq)]
enum PmpMatch { Off, Tor, Na4, Napot }

/// One PMP entry — decoded from pmpcfg + pmpaddr CSRs.
#[derive(Clone, Copy)]
struct PmpEntry {
    addr: usize,    // decoded address (shifted/masked per mode)
    mask: usize,    // for NAPOT: address mask
    cfg: u8,        // raw pmpcfg byte: L|00|A[1:0]|X|W|R
}

impl PmpEntry {
    fn locked(self) -> bool  { self.cfg & 0x80 != 0 }
    fn match_mode(self) -> PmpMatch {
        match (self.cfg >> 3) & 3 {
            0 => PmpMatch::Off,
            1 => PmpMatch::Tor,
            2 => PmpMatch::Na4,
            3 => PmpMatch::Napot,
            _ => unreachable!(),
        }
    }
    fn r(self) -> bool { self.cfg & 1 != 0 }
    fn w(self) -> bool { self.cfg & 2 != 0 }
    fn x(self) -> bool { self.cfg & 4 != 0 }
}

pub struct Pmp {
    entries: [PmpEntry; PMP_COUNT],
    count: usize,  // number of entries with A != Off (optimization)
}

impl Pmp {
    pub fn new() -> Self { ... }

    /// Update cached entry. Called on pmpcfg/pmpaddr CSR write.
    pub fn update(&mut self, index: usize, cfg: u8, addr: usize) { ... }

    /// Check physical address access. Returns Err(BadAddress) on deny.
    pub fn check(
        &self,
        paddr: usize,
        op: MemOp,
        priv_mode: PrivilegeMode,
    ) -> XResult {
        // M-mode: PMP only enforced if entry is Locked
        // S/U-mode: scan entries, first match wins
        // No match in S/U-mode → Err(XError::BadAddress) (spec §3.7.1)
        ...
    }
}
```

**Key spec rules** (§3.7.1):
- Entries checked in order 0..N; first match determines permission
- M-mode bypasses all entries unless Locked (L=1)
- S/U-mode: if no entry matches → **deny** (access fault)
- PMP applies to page-table walks at effective privilege S
- TOR (Top Of Range): matches `pmpaddr[i-1] <= addr < pmpaddr[i]`
- NAPOT: matches `addr & mask == pmpaddr & mask`
- NA4: matches exactly 4 bytes at `pmpaddr`

**PMP is per-hart** — each core has its own PMP config. In multi-core,
PMP entries are not shared (unlike Bus which is shared via Arc).

### 8. TLB

```rust
// xcore/src/cpu/riscv/mmu.rs (or mmu/tlb.rs)

const TLB_SIZE: usize = 64;

#[derive(Clone, Copy, Default)]
struct TlbEntry {
    vpn: usize,
    ppn: usize,
    asid: u16,
    perm: u8,       // bits: R=0, W=1, X=2, U=3, G=4
    level: u8,
    valid: bool,
}

impl TlbEntry {
    fn from_pte(pte: Pte, vaddr: VirtAddr, level: usize, asid: u16) -> Self {
        Self {
            vpn: vaddr.as_usize() >> PAGE_SHIFT,
            ppn: pte.ppn(),
            asid,
            perm: pte.perm_bits(),
            level: level as u8,
            valid: true,
        }
    }

    fn check_perm(&self, op: MemOp, priv_mode: PrivilegeMode,
                   sum: bool, mxr: bool) -> bool {
        let (r, w, x, u) = (self.perm & 1 != 0, self.perm & 2 != 0,
                             self.perm & 4 != 0, self.perm & 8 != 0);

        let perm_ok = match op {
            MemOp::Fetch => x,
            MemOp::Load  => r || (mxr && x),
            _            => w,
        };

        let priv_ok = if u {
            priv_mode == PrivilegeMode::User
                || (priv_mode == PrivilegeMode::Supervisor && sum)
        } else {
            priv_mode != PrivilegeMode::User
        };

        perm_ok && priv_ok
    }

    fn translate(&self, vaddr: VirtAddr, sv: &SvMode) -> usize {
        if self.level > 0 {
            let mask = (1 << (self.level as usize * sv.vpn_bits + PAGE_SHIFT)) - 1;
            (self.ppn << PAGE_SHIFT) & !mask | (vaddr.as_usize() & mask)
        } else {
            self.ppn << PAGE_SHIFT | (vaddr.as_usize() & (PAGE_SIZE - 1))
        }
    }
}

struct Tlb {
    entries: [TlbEntry; TLB_SIZE],
}

impl Tlb {
    fn new() -> Self {
        Self { entries: [TlbEntry::default(); TLB_SIZE] }
    }

    fn index(vpn: usize) -> usize {
        vpn & (TLB_SIZE - 1)
    }

    fn lookup(&self, vpn: usize, asid: u16) -> Option<&TlbEntry> {
        let entry = &self.entries[Self::index(vpn)];
        if entry.valid && entry.vpn == vpn && (entry.asid == asid || entry.perm & 16 != 0) {
            Some(entry)
        } else {
            None
        }
    }

    fn insert(&mut self, entry: TlbEntry) {
        self.entries[Self::index(entry.vpn)] = entry;
    }

    /// Flush per sfence.vma semantics.
    fn flush(&mut self, vpn: Option<usize>, asid: Option<u16>) {
        for entry in &mut self.entries {
            if !entry.valid { continue; }
            let vpn_match = vpn.map_or(true, |v| entry.vpn == v);
            let asid_match = asid.map_or(true, |a| entry.asid == a);
            let is_global = entry.perm & 16 != 0;

            let should_flush = match (vpn, asid) {
                (None, None) => true,                      // flush all
                (Some(_), None) => vpn_match,              // flush by vaddr (incl. global)
                (None, Some(_)) => asid_match && !is_global, // flush by ASID (skip global)
                (Some(_), Some(_)) => vpn_match && asid_match && !is_global,
            };

            if should_flush {
                entry.valid = false;
            }
        }
    }
}
```

Matches KXemu's `TLBBlock` + `tlb_hit`/`tlb_push`/`tlb_fence`. Direct-mapped, indexed by
lower VPN bits. ASID-tagged. Global pages (G=1) only flushed when `asid == None`.

### 9. Effective privilege (MPRV)

```rust
/// MPRV: when set in M-mode, data accesses use MPP's privilege level.
/// Instruction fetch always uses current privilege.
fn effective_priv(&self) -> PrivilegeMode {
    if self.privilege == PrivilegeMode::Machine && self.csr.mstatus().mprv() {
        self.csr.mstatus().mpp()
    } else {
        self.privilege
    }
}
```

Called for data accesses (load/store/AMO), NOT for instruction fetch.
Fetch always uses `self.privilege` directly (see §4 `fetch` code).

### 10. sfence.vma

```
INSTPAT("0001001 ????? ????? 000 00000 1110011", sfence_vma, R);
```

Handler in `inst/privileged.rs`:

```rust
pub(super) fn sfence_vma(&mut self, inst: u32) -> XResult {
    // U-mode: illegal instruction
    if self.privilege == PrivilegeMode::User {
        return self.illegal_inst();
    }
    // S-mode + TVM: illegal instruction
    if self.privilege == PrivilegeMode::Supervisor && self.csr.mstatus().tvm() {
        return self.illegal_inst();
    }

    let rs1 = reg(inst, 19, 15)?;
    let rs2 = reg(inst, 24, 20)?;
    let vpn = if rs1 != RVReg::zero { Some(self.gpr[rs1] as usize >> PAGE_SHIFT) } else { None };
    let asid = if rs2 != RVReg::zero { Some(self.gpr[rs2] as u16) } else { None };
    self.mmu.tlb.flush(vpn, asid);
    Ok(())
}
```

**satp write side effect** (in `csr/ops.rs`): When satp is written, flush TLB:
```rust
// Inside CSR write handler, after writing satp:
self.mmu.tlb.flush(None, None);
```

---

## Implementation Order

### Step 0: Device trait + Ram + Bus (new code only)

**Files created**:
- NEW: `xcore/src/device/mod.rs` — `Device` trait + `pub mod bus; pub mod ram;`
- NEW: `xcore/src/device/bus.rs` — `Bus` with RAM/MMIO split, `read`/`write`/`read_ram`/`load_ram`
- NEW: `xcore/src/device/ram.rs` — `Ram` (direct struct, not `impl Device`)

**No existing code changed.** Pure addition. Write unit tests for Bus + Ram in isolation.

Test cases:
1. `Ram::read`/`write` round-trip for 1/2/4/8-byte sizes
2. `Bus::read`/`write` hits RAM within bounds
3. `Bus::read` returns `BadAddress` for unmapped address
4. `Bus::read_ram` succeeds for RAM, fails for MMIO address
5. `Bus::load_ram` bulk loads bytes correctly
6. `Bus::add_mmio` panics on overlap with RAM or existing MMIO

### Step 1: Wire Bus into CPU via `Arc<Mutex<Bus>>` (replace global MEMORY)

**Files changed**:
- MODIFY: `xcore/src/cpu/mod.rs` — add `bus: Arc<Mutex<Bus>>` to `CPU`, clone to core
- MODIFY: `xcore/src/cpu/core.rs` — `CoreOps::step(&mut self)` (no bus param)
- MODIFY: `xcore/src/cpu/riscv/mod.rs` — add `bus: Arc<Mutex<Bus>>` field, `bus()` accessor
- MODIFY: `xcore/src/cpu/riscv/mem.rs` — rewrite: `self.bus().read()`/`.write()` instead of `with_mem!`
- MODIFY: `xcore/src/cpu/riscv/inst/base.rs` — `load_op`/`store_op` call `self.bus()` internally
- MODIFY: `xcore/src/cpu/riscv/inst/atomic.rs` — `amo_w`/`amo_d`/`lr`/`sc` call `self.bus()`
- MODIFY: `xcore/src/cpu/riscv/inst/compressed.rs` — compressed load/store call `self.load_op`/`self.store_op`
- DELETE: `xcore/src/memory/mod.rs` — replaced by device/bus + device/ram
- DELETE: `xcore/src/cpu/mem.rs` — `MemOps` trait deleted entirely

Bus shared as `Arc<Mutex<Bus>>` — no type alias.

**Migration mechanics**: `with_mem!(read(addr, size))` → `self.bus().read(addr, size)`.
`with_mem!(write(addr, size, val))` → `self.bus().write(addr, size, val)`.
`with_mem!(load_img(addr, data))` → `bus.lock().unwrap().load_ram(addr, data)`.

**Alignment checks move UP**: Currently `Memory::read/write` checks alignment.
After this step, `Ram` does raw byte access only. Alignment enforcement lives in
`RVCore::fetch` (IALIGN=16), `RVCore::load_op/store_op` (natural alignment).
`Ram` never returns `AddrNotAligned`.

**No bus parameter on dispatch or instruction handlers.** Only `fetch`, `load_op`,
`store_op`, `amo_w`, `amo_d`, `lr_*`, `sc_*` call `self.bus()`. The dispatch macro
and all ~55 ALU/branch/CSR handlers remain unchanged from their original signatures.

**Test migration**: `RVCore::new()` creates its own `Arc<Mutex<Bus>>`.
Tests use `core.bus.lock().unwrap()` to read/write memory directly.

**Zero behavioral change.** All 200 tests pass after this step.

### Step 2: MMU + PMP skeletons (Bare mode, no PMP entries)

**Files changed**:
- NEW: `xcore/src/cpu/riscv/mmu.rs` — `Mmu` (cached config), `MemOp`, `SvMode`, `Tlb` (stub)
- NEW: `xcore/src/cpu/riscv/pmp.rs` — `Pmp` (16 entries, all Off initially)
- MODIFY: `xcore/src/cpu/riscv/mod.rs` — add `mmu: Mmu` + `pmp: Pmp` to `RVCore`, add `translate`/`effective_priv`/`mem_fault_to_trap`
- MODIFY: `xcore/src/cpu/riscv/mem.rs` — `fetch`/`load`/`store` call `self.translate(vaddr, kind, priv)` instead of identity mapping

In Bare mode (default) with no PMP entries, both pass through:
- `Mmu::translate` returns identity `Ok(vaddr.as_usize())`
- `Pmp::check` returns `Ok(())` (M-mode, no locked entries)

The full path established: alignment → lock bus → MMU → PMP → bus access.

**Zero behavioral change.** M-mode with Bare satp = current behavior.

### Step 3: Page walk (SV32 / SV39) + PMP on PTE reads

**Files changed**:
- MODIFY: `xcore/src/cpu/riscv/mmu.rs` — add `page_walk`, `check_leaf_perm`, `Pte`, `PteFlags`
- MODIFY: `xcore/src/cpu/riscv/pmp.rs` — implement `Pmp::check` (TOR, NA4, NAPOT matching)
- MODIFY: `xcore/src/cpu/riscv/csr/ops.rs` — satp write → `mmu.update_satp()`, mstatus write → `mmu.update_mstatus()`, pmpcfg/pmpaddr write → `pmp.update()`

Page walk calls `pmp.check(pte_addr, Load, Supervisor)` before each PTE read (spec §3.7.1).
Final paddr checked by `RVCore::translate()` after MMU returns.

**Test plan**: Create a Bus, manually write page tables into RAM,
call `mmu.translate` and verify results.

Test cases (MMU):
1. Single-level mapping (4 KB page)
2. Superpage (4 MB for SV32, 2 MB for SV39)
3. Invalid PTE (V=0) → PageFault
4. Permission violation (write to read-only) → PageFault
5. Superpage misalignment → PageFault
6. Svade: A=0 → PageFault, D=0 on write → PageFault
7. SV39 canonical address violation → PageFault
8. U-bit: U-mode access to S-page → PageFault
9. SUM: S-mode access to U-page with SUM=0 → fault, SUM=1 → OK
10. MXR: read from X-only page with MXR=0 → fault, MXR=1 → OK
11. PTE address in MMIO range → BadAddress

Test cases (PMP):
12. No PMP entries → M-mode allowed, S/U-mode denied (no match → BadAddress)
13. NAPOT region with R+W → S-mode read/write OK, execute denied
14. TOR region boundaries: addr at low bound OK, at high bound denied
15. Locked entry enforced in M-mode
16. PMP denies PTE read during page walk → BadAddress

### Step 4: TLB + sfence.vma

**Files changed**:
- MODIFY: `xcore/src/cpu/riscv/mmu.rs` — implement `Tlb` (currently empty stub)
- MODIFY: `xcore/src/cpu/riscv/inst/privileged.rs` — add `sfence_vma`
- MODIFY: `xcore/src/isa/instpat/riscv.instpat` — add sfence.vma pattern
- MODIFY: `xcore/src/utils/macros.rs` — add `sfence_vma` to instruction table
- MODIFY: `xcore/src/cpu/riscv/csr/ops.rs` — add satp write side effect (TLB flush)

Test cases:
1. TLB hit returns cached translation
2. TLB miss triggers page walk
3. `flush(None, None)` clears all entries
4. `flush(Some(vpn), None)` clears only matching VPN
5. `flush(None, Some(asid))` clears matching ASID, preserves global
6. sfence.vma in U-mode → IllegalInst (via `self.illegal_inst()`)
7. sfence.vma in S-mode + TVM=1 → IllegalInst
8. sfence.vma in S-mode + TVM=0 → TLB flushed

### Step 5: MMIO address map

**Files changed**:
- MODIFY: `xcore/src/config/mod.rs` — add device address constants

```rust
// Standard RISC-V memory map (compatible with QEMU virt machine)
pub const CLINT_BASE: usize = 0x0200_0000;
pub const CLINT_SIZE: usize = 0x1_0000;
pub const PLIC_BASE:  usize = 0x0c00_0000;
pub const PLIC_SIZE:  usize = 0x400_0000;
pub const UART_BASE:  usize = 0xa000_03f8;
pub const UART_SIZE:  usize = 0x8;
```

Phase 4 adds devices: `bus.add_mmio("uart", UART_BASE, UART_SIZE, Box::new(Uart::new()))`.

---

## Type Budget

| Type | Kind | Purpose |
|------|------|---------|
| `Device` | trait | MMIO device read/write (2 methods) |
| `Ram` | struct | Byte array, owned directly by Bus (not a Device) |
| `Bus` | struct | RAM + MMIO dispatch, `read_ram` for page walk |
| `Mmu` | struct | Cached satp/mstatus config + TLB + page walk |
| `Pmp` | struct | Cached pmpcfg/pmpaddr, physical address permission gate |
| `Tlb` | struct | Direct-mapped translation cache (64 entries) |
| `SvMode` | struct | Page table format descriptor (levels, pte_size, vpn_bits, va_bits) |
| `MemOp` | enum | Fetch / Load / Store / Amo |
| `Pte` | struct | PTE bitfield accessor (no policy) |
| `PteFlags` | bitflags | V/R/W/X/U/G/A/D flag bits |

**10 types.** `SvMode` is a simple const struct — `SV32`/`SV39`/`SV48`/`SV57` are
static constants. Adding new modes requires zero code changes to the walk.
No `MemFault` — reuse `XError::PageFault` + `XError::BadAddress`.
`Pte`/`PteFlags`/`PmpEntry` internal. `Bus` shared via `Arc<Mutex<Bus>>`.
They encapsulate PTE bit twiddling so the walk loop stays clean (~20 lines).
Pattern borrowed from arceos `GenericPTE` + asterinas `PteFlags`.

---

## Address Map

```
0x0000_0000 ┌──────────────────┐
            │   (unmapped)     │
0x0200_0000 ├──────────────────┤
            │   CLINT (64 KB)  │  Phase 4
0x0201_0000 ├──────────────────┤
            │   (unmapped)     │
0x0c00_0000 ├──────────────────┤
            │   PLIC (64 MB)   │  Phase 4
0x1000_0000 ├──────────────────┤
            │   (unmapped)     │
0x8000_0000 ├──────────────────┤
            │   RAM (128 MB)   │
0x8800_0000 ├──────────────────┤
            │   (unmapped)     │
0xa000_03f8 ├──────────────────┤
            │   UART (8 bytes) │  Phase 4
            └──────────────────┘
```

## Page Table Formats

### SV32 (RV32)

```
Virtual Address (32 bits):
┌──────────┬──────────┬──────────────┐
│ VPN[1]   │ VPN[0]   │ page offset  │
│ 10 bits  │ 10 bits  │ 12 bits      │
└──────────┴──────────┴──────────────┘

PTE (32 bits):
┌──────────────────────┬────┬─┬─┬─┬─┬─┬─┬─┬─┐
│ PPN[1:0] (22 bits)   │RSW │D│A│G│U│X│W│R│V│
└──────────────────────┴────┴─┴─┴─┴─┴─┴─┴─┴─┘

SvMode: SV32 { levels: 2, pte_size: 4, vpn_bits: 10, va_bits: 32 }
Page sizes: 4 KB (level 0), 4 MB megapage (level 1)
```

### SV39 (RV64)

```
Virtual Address (39 bits, sign-extended to 64):
┌──────────┬──────────┬──────────┬──────────────┐
│ VPN[2]   │ VPN[1]   │ VPN[0]   │ page offset  │
│ 9 bits   │ 9 bits   │ 9 bits   │ 12 bits      │
└──────────┴──────────┴──────────┴──────────────┘

PTE (64 bits):
┌─────────┬──────────────────────┬────┬─┬─┬─┬─┬─┬─┬─┬─┐
│Reserved │ PPN[2:0] (44 bits)   │RSW │D│A│G│U│X│W│R│V│
│ 10 bits │                      │    │ │ │ │ │ │ │ │ │
└─────────┴──────────────────────┴────┴─┴─┴─┴─┴─┴─┴─┴─┘

SvMode: SV39 { levels: 3, pte_size: 8, vpn_bits: 9, va_bits: 39 }
Page sizes: 4 KB (level 0), 2 MB megapage (level 1), 1 GB gigapage (level 2)
```

## PTE Permission Rules

| R | W | X | Meaning |
|---|---|---|---------|
| 0 | 0 | 0 | Pointer to next level (non-leaf) |
| 0 | 0 | 1 | Execute-only page |
| 0 | 1 | 0 | **Reserved** → page fault |
| 0 | 1 | 1 | Read + Execute |
| 1 | 0 | 0 | Read-only page |
| 1 | 0 | 1 | Read + Execute |
| 1 | 1 | 0 | Read + Write |
| 1 | 1 | 1 | Read + Write + Execute |

**MXR**: loads from X-only pages permitted. **SUM**: S-mode access to U-pages permitted.

## References

- RISC-V Privileged Spec §4.3.1 (Sv32), §4.3.2 (Sv39), §4.2.1 (satp), §9.5.1 (sfence.vma)
- KXemu `include/device/bus.hpp` + `src/cpu/riscv/memory.cpp` — bus dispatch + page walk template
- Nemu-rust `src/memory/mod.rs` — `IOMap` trait (3 methods)
- REMU `remu_state/src/bus/` — Rust bus pattern with D-cache
- arceos `page_table_entry` — `GenericPTE` trait + bitflags `PTEFlags` for RISC-V PTE abstraction
- asterinas `ostd/src/mm/page_table` — `PteTrait` + `PteScalar { Absent, PageTable, Mapped }` pattern
- [MEM_PLAN_REVIEW.md](./MEM_PLAN_REVIEW.md), [MEM_PLAN_REVIEW_GEMINI.md](./MEM_PLAN_REVIEW_GEMINI.md)
- err2trap pattern (`fix/err2trap` branch) — traps propagate as `Err(XError::Trap(...))`, caught by `trap_on_err()` in `step()`
