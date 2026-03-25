# Memory Subsystem Implementation Plan

> Phase 3 of [DEV.md](../DEV.md) — Memory Management
> Bus + MMU + TLB + MMIO routing, Rust-idiomatic, dual RV32/RV64
>
> v6 (2026-03-25): Bus shared via `Arc<Mutex<Bus>>` instead of `&mut Bus` threading.
> Changes from v5: RVCore holds `Arc<Mutex<Bus>>` clone — no bus parameter on dispatch
> or instruction handlers. CPU owns the Arc, clones to each core. Multi-core ready.
> Prior: v5 RAM/MMIO split, AccessCtx/MemFault, err2trap, MemAccess deleted.
> Reference designs: KXemu (`bus.hpp`, `memory.cpp`), Nemu-rust (`IOMap`), REMU (`Bus`).

---

## Architecture Overview

```
CPU                                      ┌──────────────────────┐
├── core: RVCore                         │  Bus                 │
│   ├── bus: Arc<Mutex<Bus>> (clone) ──► │  ├── Ram [0x8000_0000│
│   ├── mmu: Mmu  ──translate──►paddr──► │  ├── UART [Phase 4]  │
│   │   └── tlb: Tlb                     │  └── ...             │
│   ├── csr, privilege, ...              └──────────────────────┘
│   └── step(&mut self)
├── bus: Arc<Mutex<Bus>>  ◄─── shared via Arc clone
└── state, halt_pc, halt_ret
```

Three-layer responsibility split (same principle as CSR subsystem):

| Layer | Knows about | Does NOT know about |
|-------|------------|---------------------|
| `Bus` | Physical addresses, device regions | Virtual addresses, privilege, traps |
| `Mmu` | Page tables, TLB, PTE permission bits | Trap codes, which instruction triggered the access |
| `RVCore` | Everything: privilege, MPRV, trap mapping | Internal device state |

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

type SharedBus = Arc<Mutex<Bus>>;

// CPU owns the Arc, clones to core
pub struct CPU<Core: CoreOps> {
    core: Core,
    bus: SharedBus,
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
    bus: SharedBus,  // Arc clone — same Bus as CPU
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
- Multi-core ready: `CPU { cores: Vec<Core>, bus: SharedBus }` — each core gets a clone
- Bus has the same lifetime as CPU — Arc ensures this naturally
- Single-core: mutex is uncontested, ~20ns overhead per access (acceptable)
- Instruction handlers access bus via `self.bus()` like any other field

**What gets removed**: `static MEMORY`, `with_mem!` macro, `MemOps` trait.
**What stays simple**: `CoreOps::step(&mut self)` — no bus parameter.
Instruction handler signatures unchanged from original — only memory helpers
(`fetch`, `load_op`, `store_op`, `amo_w`, `lr`, `sc`) call `self.bus()`.

### 4. MMU: pure translation, returns MemFault

**Types** — replaces the earlier `Perm` + `PageFault` with a unified access context:

```rust
// xcore/src/cpu/riscv/mmu.rs

/// What kind of memory access is being performed.
/// Determines which PTE permission bits to check, and which trap to raise on failure.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccessKind { Fetch, Load, Store, Amo }

/// All context needed for address translation.
pub struct AccessCtx {
    pub kind: AccessKind,
    pub priv_mode: PrivilegeMode,
    pub sum: bool,
    pub mxr: bool,
    pub satp: Word,
}

/// MMU translation failure — ISA-agnostic. Caller maps to RISC-V trap cause.
pub enum MemFault {
    /// Page table walk failed (invalid PTE, permission, A/D, canonical, etc.)
    Page(VirtAddr),
    /// Bus access fault during page walk (PTE address hits unmapped region)
    Access(VirtAddr),
}

pub struct Mmu {
    tlb: Tlb,
}
```

`AccessKind` tells the MMU which PTE bit to check (R/W/X). `RVCore` maps `MemFault`
to the correct RISC-V trap cause (`LoadPageFault`, `StoreAccessFault`, etc.) based on
`AccessKind`. The MMU never imports trap codes.

**The translate function** — follows KXemu's `vaddr_translate_core()` pattern:

```rust
impl Mmu {
    /// Translate virtual → physical address.
    ///
    /// Returns `Err(MemFault)` on any translation failure.
    /// The caller (RVCore) maps this to the appropriate RISC-V trap.
    pub fn translate(
        &mut self,
        vaddr: VirtAddr,
        ctx: &AccessCtx,
        bus: &MutexGuard<'_, Bus>,  // lock held by caller for duration of access
    ) -> Result<PhysAddr, MemFault> {
        let mode = satp_mode(ctx.satp);

        // Bare mode or M-mode (without MPRV changing effective priv): identity mapping
        if mode == SatpMode::Bare || ctx.priv_mode == PrivilegeMode::Machine {
            return Ok(PhysAddr::from(vaddr.as_usize()));
        }

        // TLB lookup
        let vpn = vaddr_vpn(vaddr);
        let asid = satp_asid(ctx.satp);
        if let Some(entry) = self.tlb.lookup(vpn, asid) {
            if entry.check_perm(ctx) {
                return Ok(entry.translate(vaddr));
            }
        }

        // TLB miss → page walk (reads RAM only, never MMIO)
        let result = self.page_walk(vaddr, ctx, bus)?;
        self.tlb.insert(result.entry);
        Ok(result.paddr)
    }
}
```

**Page walk uses `bus.read_ram()`**, not `bus.read()`. The caller holds the
`MutexGuard<Bus>` and passes it to `translate` → `page_walk`. A misconfigured
`satp` pointing at MMIO space returns `MemFault::Access`, never triggers a
spurious device read.

**RVCore maps MemFault to RISC-V traps** — using the err2trap `Result`-based pattern:

```rust
impl RVCore {
    /// Compute effective privilege for data access (accounts for MPRV).
    fn effective_priv(&self) -> PrivilegeMode {
        if self.privilege == PrivilegeMode::Machine && self.csr.mstatus().mprv() {
            self.csr.mstatus().mpp()
        } else {
            self.privilege
        }
    }

    /// Build AccessCtx for a data access (load/store/AMO). Uses effective privilege.
    fn data_ctx(&self, kind: AccessKind) -> AccessCtx {
        let mstatus = self.csr.mstatus();
        AccessCtx {
            kind,
            priv_mode: self.effective_priv(),
            sum: mstatus.sum(),
            mxr: mstatus.mxr(),
            satp: self.csr.get(CsrAddr::satp),
        }
    }

    /// Translate and map any MemFault to the correct RISC-V trap via Err(XError::Trap).
    fn translate(
        &mut self, vaddr: VirtAddr, ctx: &AccessCtx, bus: &MutexGuard<'_, Bus>,
    ) -> XResult<PhysAddr> {
        self.mmu.translate(vaddr, ctx, bus).map_err(|fault| {
            let (exc, tval) = match fault {
                MemFault::Page(va) => (match ctx.kind {
                    AccessKind::Fetch => Exception::InstructionPageFault,
                    AccessKind::Load  => Exception::LoadPageFault,
                    AccessKind::Store | AccessKind::Amo => Exception::StorePageFault,
                }, va.as_usize() as Word),
                MemFault::Access(va) => (match ctx.kind {
                    AccessKind::Fetch => Exception::InstructionAccessFault,
                    AccessKind::Load  => Exception::LoadAccessFault,
                    AccessKind::Store | AccessKind::Amo => Exception::StoreAccessFault,
                }, va.as_usize() as Word),
            };
            XError::Trap(PendingTrap { cause: TrapCause::Exception(exc), tval })
        })
    }
}
```

Then the load/store/fetch helpers become — using `?` to propagate traps up to
`trap_on_err()` in `step()`:

```rust
impl RVCore {
    /// Lock the shared bus for the duration of one access.
    fn bus(&self) -> MutexGuard<'_, Bus> {
        self.bus.lock().expect("bus lock poisoned")
    }

    fn fetch(&mut self) -> XResult<u32> {
        if !self.pc.is_aligned(2_usize) {
            return self.trap_exception(Exception::InstructionMisaligned,
                                       self.pc.as_usize() as Word);
        }
        let ctx = AccessCtx {
            kind: AccessKind::Fetch,
            priv_mode: self.privilege,
            sum: self.csr.mstatus().sum(),
            mxr: self.csr.mstatus().mxr(),
            satp: self.csr.get(CsrAddr::satp),
        };
        let bus = self.bus();
        let paddr = self.translate(self.pc, &ctx, &bus)?;
        let word = bus.read(paddr.as_usize(), 4)
            .map_err(|_| XError::Trap(PendingTrap {
                cause: TrapCause::Exception(Exception::InstructionAccessFault),
                tval: self.pc.as_usize() as Word,
            }))?;
        let inst = word_to_u32(word);
        Ok(if (inst & 0b11) != 0b11 { inst & 0xFFFF } else { inst })
    }

    fn load_op<F>(
        &mut self, rd: RVReg, rs1: RVReg, imm: SWord, size: usize,
        extend: F,
    ) -> XResult
    where F: FnOnce(Word) -> Word {
        let vaddr = self.eff_addr(rs1, imm);
        if !vaddr.is_aligned(size) {
            return self.trap_exception(Exception::LoadMisaligned,
                                       vaddr.as_usize() as Word);
        }
        let ctx = self.data_ctx(AccessKind::Load);
        let bus = self.bus();
        let paddr = self.translate(vaddr, &ctx, &bus)?;
        let value = bus.read(paddr.as_usize(), size)
            .map_err(|_| XError::Trap(PendingTrap {
                cause: TrapCause::Exception(Exception::LoadAccessFault),
                tval: vaddr.as_usize() as Word,
            }))?;
        self.set_gpr(rd, extend(value))
    }

    fn store_op(
        &mut self, rs1: RVReg, rs2: RVReg, imm: SWord, size: usize,
    ) -> XResult {
        let vaddr = self.eff_addr(rs1, imm);
        if !vaddr.is_aligned(size) {
            return self.trap_exception(Exception::StoreMisaligned,
                                       vaddr.as_usize() as Word);
        }
        let ctx = self.data_ctx(AccessKind::Store);
        let bus = self.bus();
        let paddr = self.translate(vaddr, &ctx, &bus)?;
        let mask = if size >= std::mem::size_of::<Word>() { Word::MAX }
                   else { (1 as Word).wrapping_shl(size as u32 * 8) - 1 };
        bus.write(paddr.as_usize(), size, self.gpr[rs2] & mask)
            .map_err(|_| XError::Trap(PendingTrap {
                cause: TrapCause::Exception(Exception::StoreAccessFault),
                tval: vaddr.as_usize() as Word,
            }))?;
        self.reservation = None;
        Ok(())
    }
}
```

**Key pattern**: Alignment → lock bus → translate → bus access → drop lock.
Each layer can fail independently via `Err(XError::Trap(...))` + `?`.
The bus lock is held for the minimum scope (one translate+access pair).

**No `bus` parameter on instruction handlers.** `lb`, `sw`, `amoadd_w` etc. keep
their original signatures `(&mut self, rd, rs1, imm) -> XResult`. Only the
internal helpers (`fetch`, `load_op`, `store_op`, `amo_w`, `lr_w`, `sc_w`) call
`self.bus()`. ALU / branch / CSR handlers never touch the bus.

Same three-level chain as KXemu's `vm_read` → `vaddr_translate_core` → `pm_read_check`.

### 5. Page table walk

Follows the RISC-V spec algorithm (§4.3.2 Sv39 / §4.3.1 Sv32). Inspired by
arceos/page_table_entry's `GenericPTE` and asterinas's `PteScalar` pattern:
extract all PTE bit manipulation into a dedicated `Pte` type so the walk loop
stays clean.

#### 5a. Pte type — encapsulates all bit twiddling

```rust
// xcore/src/cpu/riscv/mmu.rs

const PAGE_SHIFT: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

#[cfg(isa32)]
mod sv { pub const LEVELS: usize = 2; pub const PTESIZE: usize = 4; pub const VPN_BITS: usize = 10; }
#[cfg(isa64)]
mod sv { pub const LEVELS: usize = 3; pub const PTESIZE: usize = 8; pub const VPN_BITS: usize = 9; }
use sv::*;

bitflags::bitflags! {
    /// PTE flag bits (RISC-V Privileged Spec §4.3).
    struct PteFlags: usize {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

/// A decoded page table entry. All bit-level operations live here.
#[derive(Clone, Copy)]
struct Pte(usize);

impl Pte {
    fn flags(self) -> PteFlags { PteFlags::from_bits_truncate(self.0) }
    fn is_valid(self) -> bool { self.flags().contains(PteFlags::V) }
    fn is_leaf(self) -> bool { self.flags().intersects(PteFlags::R | PteFlags::X) }

    fn ppn(self) -> usize {
        (self.0 >> 10) & ((1 << (LEVELS * VPN_BITS + 2)) - 1)
    }

    /// Check perm + U-bit + A/D against AccessCtx. Returns true if access is allowed.
    fn check_perm(self, ctx: &AccessCtx) -> bool {
        let f = self.flags();
        let perm_ok = match ctx.kind {
            AccessKind::Load  => f.contains(PteFlags::R) || (ctx.mxr && f.contains(PteFlags::X)),
            AccessKind::Store | AccessKind::Amo => f.contains(PteFlags::W),
            AccessKind::Fetch => f.contains(PteFlags::X),
        };
        let priv_ok = if f.contains(PteFlags::U) {
            ctx.priv_mode == PrivilegeMode::User
                || (ctx.priv_mode == PrivilegeMode::Supervisor && ctx.sum)
        } else {
            ctx.priv_mode != PrivilegeMode::User
        };
        // Svade: A must be set; D must be set for writes
        let ad_ok = f.contains(PteFlags::A)
            && (!matches!(ctx.kind, AccessKind::Store | AccessKind::Amo) || f.contains(PteFlags::D));

        perm_ok && priv_ok && ad_ok
    }

    /// Check superpage alignment: lower PPN bits must be zero at level > 0.
    fn superpage_aligned(self, level: usize) -> bool {
        if level == 0 { return true; }
        (self.ppn() & ((1 << (level * VPN_BITS)) - 1)) == 0
    }

    /// Build physical address for a leaf PTE at the given level.
    fn translate(self, vaddr: VirtAddr, level: usize) -> PhysAddr {
        let pg_offset = vaddr.as_usize() & (PAGE_SIZE - 1);
        if level > 0 {
            let mask = (1 << (level * VPN_BITS + PAGE_SHIFT)) - 1;
            PhysAddr::from((self.ppn() << PAGE_SHIFT) & !mask | (vaddr.as_usize() & mask))
        } else {
            PhysAddr::from(self.ppn() << PAGE_SHIFT | pg_offset)
        }
    }

    /// Pack permission bits for TlbEntry.
    fn perm_bits(self) -> u8 {
        (self.flags() & (PteFlags::R | PteFlags::W | PteFlags::X | PteFlags::U | PteFlags::G))
            .bits() as u8 >> 1  // shift R to bit 0
    }
}
```

This follows the same principle as arceos's `GenericPTE` (bitflags + accessor
methods) and asterinas's `PteFlags` (bitflags for V/R/W/X/U/G/A/D). The key
difference: our `Pte` is read-only (emulator reads guest PTEs, never writes
them back — see Svade in §6).

#### 5b. The walk itself — now trivially short

```rust
struct WalkResult {
    paddr: PhysAddr,
    entry: TlbEntry,
}

impl Mmu {
    fn page_walk(
        &self,
        vaddr: VirtAddr,
        ctx: &AccessCtx,
        bus: &MutexGuard<'_, Bus>,
    ) -> Result<WalkResult, MemFault> {
        #[cfg(isa64)]
        if !is_canonical(vaddr) { return Err(MemFault::Page(vaddr)); }

        let mut base = satp_ppn(ctx.satp) * PAGE_SIZE;

        for level in (0..LEVELS).rev() {
            let vpn_i = vpn_index(vaddr, level);
            let pte_addr = base + vpn_i * PTESIZE;
            let pte = self.read_pte(pte_addr, bus, vaddr)?;

            if !pte.is_valid() || pte.is_reserved() {
                return Err(MemFault::Page(vaddr));
            }

            if pte.is_leaf() {
                if !pte.superpage_aligned(level) || !pte.check_perm(ctx) {
                    return Err(MemFault::Page(vaddr));
                }
                return Ok(WalkResult {
                    paddr: pte.translate(vaddr, level),
                    entry: TlbEntry::from_pte(pte, vaddr, level, satp_asid(ctx.satp)),
                });
            }

            // Non-leaf: descend to next level
            base = pte.ppn() * PAGE_SIZE;
        }

        Err(MemFault::Page(vaddr))
    }

    /// Read a PTE from RAM. Returns MemFault::Access if address is unmapped.
    fn read_pte(&self, addr: usize, bus: &MutexGuard<'_, Bus>, vaddr: VirtAddr) -> Result<Pte, MemFault> {
        bus.read_ram(addr, PTESIZE)
            .map(|w| Pte(w as usize))
            .map_err(|_| MemFault::Access(vaddr))
    }
}

/// Extract VPN[level] from a virtual address.
fn vpn_index(vaddr: VirtAddr, level: usize) -> usize {
    (vaddr.as_usize() >> (PAGE_SHIFT + level * VPN_BITS)) & ((1 << VPN_BITS) - 1)
}

impl Pte {
    /// Reserved encoding: W=1 but R=0.
    fn is_reserved(self) -> bool {
        let f = self.flags();
        f.contains(PteFlags::W) && !f.contains(PteFlags::R)
    }
}

#[cfg(isa64)]
fn is_canonical(vaddr: VirtAddr) -> bool {
    let va = vaddr.as_usize() as i64;
    // SV39: bits[63:39] must be sign-extension of bit[38]
    (va << (64 - 39)) >> (64 - 39) == va
}
```

**Compared to v4**: The walk loop dropped from ~80 lines to ~20 lines. All PTE
bit manipulation moved into `Pte` methods (which are independently testable).
Same algorithm, same spec compliance — just cleaner separation of concerns.

**Compared to reference implementations**:
- arceos `GenericPTE`: similar — `is_present()`, `is_huge()`, `paddr()`, `flags()`
- asterinas `PteTrait::to_repr()`: similar — converts raw bits to `Absent | PageTable | Mapped`
- Both: PTE type encapsulates bits, walk loop stays clean

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

### 7. TLB

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

    fn check_perm(&self, ctx: &AccessCtx) -> bool {
        let r = self.perm & 1 != 0;
        let w = self.perm & 2 != 0;
        let x = self.perm & 4 != 0;
        let u = self.perm & 8 != 0;

        let perm_ok = match ctx.kind {
            AccessKind::Load  => r || (ctx.mxr && x),
            AccessKind::Store | AccessKind::Amo => w,
            AccessKind::Fetch => x,
        };

        let priv_ok = if u {
            ctx.priv_mode == PrivilegeMode::User
                || (ctx.priv_mode == PrivilegeMode::Supervisor && ctx.sum)
        } else {
            ctx.priv_mode != PrivilegeMode::User
        };

        perm_ok && priv_ok
    }

    fn translate(&self, vaddr: VirtAddr) -> PhysAddr {
        let pg_offset = vaddr.as_usize() & (PAGE_SIZE - 1);
        if self.level > 0 {
            let mask = (1 << (self.level as usize * VPN_BITS + PAGE_SHIFT)) - 1;
            PhysAddr::from((self.ppn << PAGE_SHIFT) & !mask | (vaddr.as_usize() & mask))
        } else {
            PhysAddr::from(self.ppn << PAGE_SHIFT | pg_offset)
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

### 8. Effective privilege (MPRV)

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

Called in `translate_data` (load/store/AMO), NOT in `translate_fetch`.

### 9. sfence.vma

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
- MODIFY: `xcore/src/cpu/mod.rs` — add `bus: SharedBus` to `CPU`, clone to core
- MODIFY: `xcore/src/cpu/core.rs` — `CoreOps::step(&mut self)` (no bus param)
- MODIFY: `xcore/src/cpu/riscv/mod.rs` — add `bus: SharedBus` field, `bus()` accessor
- MODIFY: `xcore/src/cpu/riscv/mem.rs` — rewrite: `self.bus().read()`/`.write()` instead of `with_mem!`
- MODIFY: `xcore/src/cpu/riscv/inst/base.rs` — `load_op`/`store_op` call `self.bus()` internally
- MODIFY: `xcore/src/cpu/riscv/inst/atomic.rs` — `amo_w`/`amo_d`/`lr`/`sc` call `self.bus()`
- MODIFY: `xcore/src/cpu/riscv/inst/compressed.rs` — compressed load/store call `self.load_op`/`self.store_op`
- DELETE: `xcore/src/memory/mod.rs` — replaced by device/bus + device/ram
- DELETE: `xcore/src/cpu/mem.rs` — `MemOps` trait deleted entirely

**Type alias**: `pub type SharedBus = Arc<Mutex<Bus>>;` in `device/bus.rs`.

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

**Test migration**: Tests create `let bus = SharedBus::new(Bus::new(...))` and
clone into `RVCore`. Or use a helper `setup_core() -> RVCore` that creates the bus.

**Zero behavioral change.** All 200 tests pass after this step.

### Step 2: MMU skeleton (Bare mode)

**Files changed**:
- NEW: `xcore/src/cpu/riscv/mmu.rs` — `Mmu`, `AccessKind`, `AccessCtx`, `MemFault`, `Tlb` (stub), `translate`
- MODIFY: `xcore/src/cpu/riscv/mod.rs` — add `mmu: Mmu` to `RVCore`, add `translate`/`data_ctx`/`effective_priv`
- MODIFY: `xcore/src/cpu/riscv/mem.rs` — `fetch`/`load`/`store` call `self.translate()` instead of identity mapping

In Bare mode, `translate` returns identity: `Ok(PhysAddr::from(vaddr.as_usize()))`.
The alignment-check → lock bus → translate → bus-access pattern shown in §4 is established here.

**Zero behavioral change.** M-mode with Bare satp = current behavior.

### Step 3: Page walk (SV32 / SV39)

**Files changed**:
- MODIFY: `xcore/src/cpu/riscv/mmu.rs` — add `page_walk`, `satp_mode`/`satp_ppn`/`satp_asid` helpers

**Page walk uses `bus.read_ram()`**, not `bus.read()`. A misconfigured satp pointing
at MMIO returns `MemFault::Access`, never triggers a device read.

**Test plan**: Create a `Bus`, manually write a page table structure into RAM,
set satp, call `mmu.translate` and verify the returned physical address.

Test cases:
1. Single-level mapping (4 KB page)
2. Superpage (4 MB for SV32, 2 MB for SV39)
3. Invalid PTE (V=0) → MemFault::Page
4. Permission violation (write to read-only) → MemFault::Page
5. Superpage misalignment → MemFault::Page
6. Svade: A=0 → MemFault::Page, D=0 on write → MemFault::Page
7. SV39 canonical address violation → MemFault::Page
8. U-bit: U-mode access to S-page → MemFault::Page
9. SUM: S-mode access to U-page with SUM=0 → fault, SUM=1 → OK
10. MXR: read from X-only page with MXR=0 → fault, MXR=1 → OK
11. PTE address in MMIO range → MemFault::Access (not MemFault::Page)

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
| `Bus` | struct | RAM + MMIO dispatch, provides `read_ram` for page walk |
| `SharedBus` | type alias | `Arc<Mutex<Bus>>` — shared between CPU and cores |
| `Mmu` | struct | TLB + page walk |
| `Tlb` | struct | Direct-mapped translation cache (64 entries) |
| `AccessKind` | enum | Fetch / Load / Store / Amo |
| `AccessCtx` | struct | Full translation context (kind + priv + SUM/MXR + satp) |
| `MemFault` | enum | Page / Access — ISA-agnostic translation failure |
| `Pte` | struct | Decoded PTE — all bit manipulation lives here |
| `PteFlags` | bitflags | V/R/W/X/U/G/A/D flag bits |

**11 types total.** `SharedBus` is a thin alias. `Pte` + `PteFlags` are internal to `mmu.rs`.
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

LEVELS = 2, PTESIZE = 4, VPN_BITS = 10
Page sizes: 4 KB (i=0), 4 MB megapage (i=1)
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

LEVELS = 3, PTESIZE = 8, VPN_BITS = 9
Page sizes: 4 KB (i=0), 2 MB megapage (i=1), 1 GB gigapage (i=2)
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
