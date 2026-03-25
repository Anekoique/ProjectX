# Memory Subsystem Implementation Plan

> Phase 3 of [DEV.md](../DEV.md) — Memory Management
> Bus + MMU + TLB + MMIO routing, Rust-idiomatic, dual RV32/RV64
>
> v4 (2026-03-23): Full code detail. 7 new types, 5 implementation steps.
> Reference designs: KXemu (`bus.hpp`, `memory.cpp`), Nemu-rust (`IOMap`), REMU (`Bus`).

---

## Architecture Overview

```
CPU                                      ┌──────────────────────┐
├── core: RVCore                         │  Bus                 │
│   ├── mmu: Mmu  ──translate──►paddr──► │  ├── Ram [0x8000_0000│
│   │   └── tlb: Tlb                     │  ├── UART [Phase 4]  │
│   ├── csr, privilege, ...              │  └── ...             │
│   └── step(&mut self, bus: &mut Bus)   └──────────────────────┘
├── bus: Bus  ◄─── shared, passed to core via &mut
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

struct Region {
    name: &'static str,
    base: usize,
    size: usize,
    device: Box<dyn Device>,
}

pub struct Bus {
    regions: Vec<Region>,
}

impl Bus {
    pub fn new() -> Self {
        Self { regions: Vec::new() }
    }

    pub fn add_region(&mut self, name: &'static str, base: usize, size: usize, device: Box<dyn Device>) {
        assert!(size > 0, "region size must be non-zero");
        assert!(base.checked_add(size).is_some(), "region overflows address space");
        for r in &self.regions {
            let no_overlap = base + size <= r.base || r.base + r.size <= base;
            assert!(no_overlap, "region '{}' [{:#x}..{:#x}) overlaps '{}'", name, base, base + size, r.name);
        }
        self.regions.push(Region { name, base, size, device });
    }

    fn find_region(&mut self, addr: usize, size: usize) -> XResult<(&mut dyn Device, usize)> {
        for r in &mut self.regions {
            if addr >= r.base && addr + size <= r.base + r.size {
                return Ok((r.device.as_mut(), addr - r.base));
            }
        }
        Err(XError::BadAddress)
    }

    pub fn read(&mut self, addr: usize, size: usize) -> XResult<Word> {
        let (dev, offset) = self.find_region(addr, size)?;
        dev.read(offset, size)
    }

    pub fn write(&mut self, addr: usize, size: usize, value: Word) -> XResult {
        let (dev, offset) = self.find_region(addr, size)?;
        dev.write(offset, size, value)
    }

    /// Bulk load bytes into a region (for image loading).
    pub fn load(&mut self, addr: usize, data: &[u8]) -> XResult {
        // Write byte-by-byte through device interface, or downcast to Ram for bulk copy.
        // Implementation detail — optimize later if needed.
        for (i, &byte) in data.iter().enumerate() {
            self.write(addr + i, 1, byte as Word)?;
        }
        Ok(())
    }
}
```

**Linear scan** (`find_region`): ~5 devices, O(1) in practice. KXemu, REMU, Nemu-rust
all use linear scan. No need for interval trees.

**`Box<dyn Device>` for RAM too**: The vtable call overhead (~2ns) is negligible in a
functional emulator. KXemu uses virtual dispatch for `MemoryBlock`. Avoiding premature
optimization keeps the bus dead simple.

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
}

impl Device for Ram {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        let mut buf = [0u8; std::mem::size_of::<Word>()];
        buf[..size].copy_from_slice(&self.data[offset..offset + size]);
        Ok(Word::from_le_bytes(buf))
    }

    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult {
        let bytes = value.to_le_bytes();
        self.data[offset..offset + size].copy_from_slice(&bytes[..size]);
        Ok(())
    }
}
```

**No alignment checks in Ram.** Ram does raw byte access. Architectural alignment rules
are enforced by RVCore before the bus call (see §3). This prevents the "bus returns
`AddrNotAligned`, caller guesses which trap" problem.

The `read`/`write` logic is identical to current `Memory::read_at`/`Memory::write` but
without the alignment and bounds checks (the bus already validated the region bounds in
`find_region`).

### 3. Bus ownership: in CPU, explicit `&mut Bus`

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
// Bus lives in CPU, no global state
pub struct CPU<Core: CoreOps> {
    core: Core,
    bus: Bus,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
}

// CoreOps::step receives &mut Bus
pub trait CoreOps {
    fn step(&mut self, bus: &mut Bus) -> XResult;
    ...
}

// CPU::step split-borrows core and bus — no borrow conflict
impl<Core: CoreOps> CPU<Core> {
    pub fn step(&mut self) -> XResult {
        self.core.step(&mut self.bus)?;
        if self.core.halted() {
            self.set_terminated(State::HALTED).log_termination();
        }
        Ok(())
    }
}

// RVCore calls bus directly — no mutex, no macro
fn fetch(&mut self, bus: &mut Bus) -> XResult<u32> {
    let paddr = self.translate_fetch(self.pc, bus)?;
    let word = bus.read(paddr, 4)?;
    ...
}
```

This is exactly KXemu's pattern: `RVCPU` holds `Bus*`, passes it to `RVCore::step()`.
The core never owns or acquires the bus — it receives a mutable reference from its caller.

**What gets removed**: `static MEMORY`, `with_mem!` macro, the `Mutex` around memory.
**What gets removed from traits**: `MemOps` trait is no longer needed — `virt_to_phys`
moves into `Mmu::translate`, `init_memory` moves into `CPU::load`.

### 4. MMU: pure translation, returns PageFault

```rust
// xcore/src/cpu/riscv/mmu.rs

/// Permission the PTE leaf must grant. Maps to PTE R/W/X bits.
#[derive(Clone, Copy)]
pub enum Perm { R, W, X }

/// MMU translation failure — always becomes a page fault trap.
pub struct PageFault {
    pub vaddr: VirtAddr,
}

pub struct Mmu {
    tlb: Tlb,
}
```

**The translate function** — follows KXemu's `vaddr_translate_core()` pattern:

```rust
impl Mmu {
    /// Translate virtual → physical address.
    ///
    /// Returns `Err(PageFault)` on any translation failure.
    /// The caller (RVCore) maps this to the appropriate trap code
    /// (InstructionPageFault / LoadPageFault / StorePageFault).
    pub fn translate(
        &mut self,
        vaddr: VirtAddr,
        perm: Perm,
        priv_mode: PrivilegeMode,
        sum: bool,
        mxr: bool,
        satp: Word,
        bus: &mut Bus,
    ) -> Result<PhysAddr, PageFault> {
        let mode = satp_mode(satp);

        // Bare mode or M-mode: identity mapping
        if mode == SatpMode::Bare || priv_mode == PrivilegeMode::Machine {
            return Ok(PhysAddr::from(vaddr.as_usize()));
        }

        // TLB lookup
        let vpn = vaddr_vpn(vaddr);
        let asid = satp_asid(satp);
        if let Some(entry) = self.tlb.lookup(vpn, asid) {
            if entry.check_perm(perm, priv_mode, sum, mxr) {
                return Ok(entry.translate(vaddr));
            }
        }

        // TLB miss → page walk
        let result = self.page_walk(vaddr, perm, priv_mode, sum, mxr, satp, bus)?;
        self.tlb.insert(result.entry);
        Ok(result.paddr)
    }
}
```

**RVCore uses it like this** — mirroring KXemu's `vm_read`/`vm_write`/`vm_ifetch`:

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

    /// Translate a data address (load/store/AMO). Uses effective privilege.
    fn translate_data(
        &mut self, vaddr: VirtAddr, perm: Perm, bus: &mut Bus,
    ) -> Result<PhysAddr, ()> {
        let priv_mode = self.effective_priv();
        let mstatus = self.csr.mstatus();
        self.mmu.translate(vaddr, perm, priv_mode, mstatus.sum(), mstatus.mxr(),
                           self.csr.get(CsrAddr::satp), bus)
            .map_err(|pf| {
                let cause = match perm {
                    Perm::R => Exception::LoadPageFault,
                    Perm::W => Exception::StorePageFault,
                    Perm::X => unreachable!(),
                };
                self.raise_trap(TrapCause::Exception(cause), pf.vaddr.as_usize() as Word);
            })
    }

    /// Translate an instruction fetch. Always uses current privilege (not MPRV).
    fn translate_fetch(
        &mut self, vaddr: VirtAddr, bus: &mut Bus,
    ) -> Result<PhysAddr, ()> {
        let mstatus = self.csr.mstatus();
        self.mmu.translate(vaddr, Perm::X, self.privilege, mstatus.sum(), mstatus.mxr(),
                           self.csr.get(CsrAddr::satp), bus)
            .map_err(|pf| {
                self.raise_trap(TrapCause::Exception(Exception::InstructionPageFault),
                                pf.vaddr.as_usize() as Word);
            })
    }
}
```

Then the load/store/fetch helpers become:

```rust
impl RVCore {
    fn fetch(&mut self, bus: &mut Bus) -> XResult<u32> {
        // Alignment: IALIGN=16, instruction fetch requires 2-byte alignment
        if !self.pc.is_aligned(2_usize) {
            self.raise_trap(TrapCause::Exception(Exception::InstructionAddrMisaligned),
                            self.pc.as_usize() as Word);
            return Ok(0); // will be consumed by pending_trap in retire
        }
        let paddr = match self.translate_fetch(self.pc, bus) {
            Ok(pa) => pa,
            Err(()) => return Ok(0),
        };
        let word = bus.read(paddr.as_usize(), 4).map_err(|_| {
            self.raise_trap(TrapCause::Exception(Exception::InstructionAccessFault),
                            self.pc.as_usize() as Word);
            XError::BadAddress
        })?;
        let inst = word_to_u32(word);
        Ok(if (inst & 0b11) != 0b11 { inst & 0xFFFF } else { inst })
    }

    fn load_with<F>(
        &mut self, rd: RVReg, rs1: RVReg, imm: SWord, size: usize,
        extend: F, bus: &mut Bus,
    ) -> XResult
    where F: FnOnce(Word) -> Word {
        let vaddr = self.eff_addr(rs1, imm);
        // Alignment check
        if !vaddr.is_aligned(size) {
            self.raise_trap(TrapCause::Exception(Exception::LoadAddrMisaligned),
                            vaddr.as_usize() as Word);
            return Ok(());
        }
        let paddr = match self.translate_data(vaddr, Perm::R, bus) {
            Ok(pa) => pa,
            Err(()) => return Ok(()),
        };
        let value = bus.read(paddr.as_usize(), size).map_err(|_| {
            self.raise_trap(TrapCause::Exception(Exception::LoadAccessFault),
                            vaddr.as_usize() as Word);
            XError::BadAddress
        })?;
        self.set_gpr(rd, extend(value))
    }

    fn store(
        &mut self, rs1: RVReg, rs2: RVReg, imm: SWord, size: usize,
        bus: &mut Bus,
    ) -> XResult {
        let vaddr = self.eff_addr(rs1, imm);
        if !vaddr.is_aligned(size) {
            self.raise_trap(TrapCause::Exception(Exception::StoreAddrMisaligned),
                            vaddr.as_usize() as Word);
            return Ok(());
        }
        let paddr = match self.translate_data(vaddr, Perm::W, bus) {
            Ok(pa) => pa,
            Err(()) => return Ok(()),
        };
        let mask = if size >= std::mem::size_of::<Word>() { Word::MAX }
                   else { (1 as Word).wrapping_shl(size as u32 * 8) - 1 };
        bus.write(paddr.as_usize(), size, self.gpr[rs2] & mask).map_err(|_| {
            self.raise_trap(TrapCause::Exception(Exception::StoreAccessFault),
                            vaddr.as_usize() as Word);
            XError::BadAddress
        })?;
        self.reservation = None;
        Ok(())
    }
}
```

**Key pattern**: Alignment → translate → bus access. Each layer can fail independently:
- Alignment fail → misaligned trap
- Translation fail → page fault trap
- Bus fail → access fault trap

Same three-level chain as KXemu's `vm_read` → `vaddr_translate_core` → `pm_read_check`.

### 5. Page table walk

Follows the RISC-V spec algorithm (§4.3.2 Sv39 / §4.3.1 Sv32) and KXemu's
`vaddr_translate_sv<LEVELS, PTESIZE, VPNBITS>` template:

```rust
const PAGE_SHIFT: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

#[cfg(isa32)]
const LEVELS: usize = 2;
#[cfg(isa32)]
const PTESIZE: usize = 4;
#[cfg(isa32)]
const VPN_BITS: usize = 10;

#[cfg(isa64)]
const LEVELS: usize = 3;
#[cfg(isa64)]
const PTESIZE: usize = 8;
#[cfg(isa64)]
const VPN_BITS: usize = 9;

struct WalkResult {
    paddr: PhysAddr,
    entry: TlbEntry,
}

impl Mmu {
    fn page_walk(
        &self,
        vaddr: VirtAddr,
        perm: Perm,
        priv_mode: PrivilegeMode,
        sum: bool,
        mxr: bool,
        satp: Word,
        bus: &mut Bus,
    ) -> Result<WalkResult, PageFault> {
        let fault = || Err(PageFault { vaddr });

        // SV39 canonical check: bits[63:39] must all equal bit[38]
        #[cfg(isa64)]
        {
            let va = vaddr.as_usize() as u64;
            let top = va >> 38;
            if top != 0 && top != (1 << (64 - 38)) - 1 {
                return fault();
            }
        }

        let mut base = satp_ppn(satp) * PAGE_SIZE;

        for i in (0..LEVELS).rev() {
            let vpn_i = (vaddr.as_usize() >> (PAGE_SHIFT + i * VPN_BITS)) & ((1 << VPN_BITS) - 1);
            let pte_addr = base + vpn_i * PTESIZE;

            // Read PTE from physical memory via bus
            let pte = bus.read(pte_addr, PTESIZE).map_err(|_| PageFault { vaddr })?
                      as usize;

            let v = pte & 1 != 0;
            let r = pte & 2 != 0;
            let w = pte & 4 != 0;
            let x = pte & 8 != 0;
            let u = pte & 16 != 0;
            let g = pte & 32 != 0;
            let a = pte & 64 != 0;
            let d = pte & 128 != 0;

            // Invalid or reserved encoding
            if !v || (!r && w) {
                return fault();
            }

            if r || x {
                // ── Leaf PTE found ──

                // Superpage alignment: lower PPN bits must be zero
                if i > 0 {
                    let lower_ppn = (pte >> 10) & ((1 << (i * VPN_BITS)) - 1);
                    if lower_ppn != 0 {
                        return fault();
                    }
                }

                // Permission check
                let ok = match perm {
                    Perm::R => r || (mxr && x),
                    Perm::W => w,
                    Perm::X => x,
                };
                if !ok { return fault(); }

                // U-bit check
                if u {
                    if priv_mode == PrivilegeMode::Supervisor && !sum { return fault(); }
                    if priv_mode == PrivilegeMode::Machine { return fault(); }
                } else if priv_mode == PrivilegeMode::User {
                    return fault();
                }

                // Svade: A must be set; D must be set for writes
                if !a { return fault(); }
                if matches!(perm, Perm::W) && !d { return fault(); }

                // Build physical address
                let ppn = (pte >> 10) & ((1usize << (LEVELS * VPN_BITS + 2)) - 1);
                let pg_offset = vaddr.as_usize() & (PAGE_SIZE - 1);

                let paddr = if i > 0 {
                    // Superpage: use VPN bits from vaddr for lower levels
                    let mask = (1 << (i * VPN_BITS + PAGE_SHIFT)) - 1;
                    (ppn << PAGE_SHIFT) & !mask | (vaddr.as_usize() & mask)
                } else {
                    ppn << PAGE_SHIFT | pg_offset
                };

                let perm_bits = ((r as u8) << 0) | ((w as u8) << 1) | ((x as u8) << 2)
                              | ((u as u8) << 3) | ((g as u8) << 4);

                return Ok(WalkResult {
                    paddr: PhysAddr::from(paddr),
                    entry: TlbEntry {
                        vpn: vaddr.as_usize() >> PAGE_SHIFT,
                        ppn: ppn,
                        asid: satp_asid(satp),
                        perm: perm_bits,
                        level: i as u8,
                        valid: true,
                    },
                });
            }

            // Non-leaf PTE: descend
            let ppn = (pte >> 10) & ((1usize << (LEVELS * VPN_BITS + 2)) - 1);
            base = ppn * PAGE_SIZE;
        }

        fault()
    }
}
```

**Compared to KXemu's template** (`vaddr_translate_sv<LEVELS, PTESIZE, VPNBITS>`):
same algorithm, but we use `cfg(isa32/isa64)` constants instead of C++ template params.

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
    fn check_perm(&self, perm: Perm, priv_mode: PrivilegeMode, sum: bool, mxr: bool) -> bool {
        let r = self.perm & 1 != 0;
        let w = self.perm & 2 != 0;
        let x = self.perm & 4 != 0;
        let u = self.perm & 8 != 0;

        let perm_ok = match perm {
            Perm::R => r || (mxr && x),
            Perm::W => w,
            Perm::X => x,
        };

        let priv_ok = if u {
            priv_mode == PrivilegeMode::User || (priv_mode == PrivilegeMode::Supervisor && sum)
        } else {
            priv_mode != PrivilegeMode::User
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
pub(super) fn sfence_vma(&mut self, inst: u32, bus: &mut Bus) -> XResult {
    // U-mode: illegal instruction
    if self.privilege == PrivilegeMode::User {
        self.raise_trap(TrapCause::Exception(Exception::IllegalInst), 0);
        return Ok(());
    }
    // S-mode + TVM: illegal instruction
    if self.privilege == PrivilegeMode::Supervisor && self.csr.mstatus().tvm() {
        self.raise_trap(TrapCause::Exception(Exception::IllegalInst), 0);
        return Ok(());
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

### Step 1: Bus + Ram (replace global MEMORY)

**Files changed**:
- NEW: `xcore/src/device/mod.rs` — `Device` trait + `pub mod bus; pub mod ram;`
- NEW: `xcore/src/device/bus.rs` — `Bus`, `Region`, `add_region`, `read`/`write`/`load`
- NEW: `xcore/src/device/ram.rs` — `Ram` implementing `Device`
- MODIFY: `xcore/src/cpu/mod.rs` — add `bus: Bus` to `CPU`, change `step`/`reset`/`load`
- MODIFY: `xcore/src/cpu/core.rs` — `CoreOps::step(&mut self, bus: &mut Bus)`
- MODIFY: `xcore/src/cpu/riscv/mod.rs` — `RVCore::step` takes `&mut Bus`
- MODIFY: `xcore/src/cpu/riscv/inst/base.rs` — `load_with`/`store` take `&mut Bus`
- MODIFY: `xcore/src/cpu/riscv/inst/atomic.rs` — `amo_w`/`amo_d`/`lr`/`sc` take `&mut Bus`
- MODIFY: `xcore/src/cpu/riscv/inst/compressed.rs` — compressed load/store take `&mut Bus`
- DELETE: `xcore/src/memory/mod.rs` — replaced by device/bus + device/ram
- DELETE: `xcore/src/cpu/mem.rs` — `MemOps` trait no longer needed

**Migration mechanics**: `with_mem!(read(addr, size))` → `bus.read(addr.as_usize(), size)`.
`with_mem!(write(addr, size, val))` → `bus.write(addr.as_usize(), size, val)`.
`with_mem!(load(addr, data))` → `bus.load(addr.as_usize(), data)`.

**Bus threading**: `&mut Bus` flows from `CPU::step` → `RVCore::step` → `fetch`/`execute`
→ `dispatch` → individual instruction handlers → `load_with`/`store`.

**Test migration**: Tests that use `with_mem!` directly create a local `Bus` with `Ram`
instead. The `setup_core` helper becomes `setup_core_and_bus() -> (RVCore, Bus)`.

**Zero behavioral change.** All 196 tests pass after this step.

### Step 2: MMU skeleton (Bare mode)

**Files changed**:
- NEW: `xcore/src/cpu/riscv/mmu.rs` — `Mmu`, `Perm`, `PageFault`, `Tlb` (empty), `translate`
- MODIFY: `xcore/src/cpu/riscv/mod.rs` — add `mmu: Mmu` to `RVCore`
- MODIFY: `xcore/src/cpu/riscv/inst/base.rs` — `load_with`/`store` call `translate_data`
- MODIFY: `xcore/src/cpu/riscv/mod.rs` — `fetch` calls `translate_fetch`

In Bare mode, `translate` returns identity: `Ok(PhysAddr::from(vaddr.as_usize()))`.
RVCore's `translate_fetch` and `translate_data` add the alignment-check-then-translate-
then-bus-access pattern shown in §4.

**Zero behavioral change.** M-mode with Bare satp = current behavior.

### Step 3: Page walk (SV32 / SV39)

**Files changed**:
- MODIFY: `xcore/src/cpu/riscv/mmu.rs` — add `page_walk`, `satp_mode`/`satp_ppn`/`satp_asid` helpers

**Test plan**: Create a `Bus` with `Ram`, manually write a page table structure into RAM,
set satp to point at it, call `mmu.translate` and verify the returned physical address.

Test cases:
1. Single-level mapping (4 KB page)
2. Superpage (4 MB for SV32, 2 MB for SV39)
3. Invalid PTE (V=0) → PageFault
4. Permission violation (write to read-only) → PageFault
5. Superpage misalignment → PageFault
6. Svade: A=0 → PageFault, D=0 on write → PageFault
7. SV39 canonical address violation → PageFault
8. U-bit: U-mode access to S-page → PageFault
9. SUM: S-mode access to U-page with SUM=0 → PageFault, SUM=1 → OK
10. MXR: read from X-only page with MXR=0 → PageFault, MXR=1 → OK

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
6. sfence.vma in U-mode → IllegalInst
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

Phase 4 adds devices: `bus.add_region("uart", UART_BASE, UART_SIZE, Box::new(Uart::new()))`.

---

## Type Budget

| Type | Kind | Purpose |
|------|------|---------|
| `Device` | trait | Physical bus-mapped read/write (2 methods) |
| `Ram` | struct | Byte array implementing Device |
| `Bus` | struct | Physical address dispatch to regions |
| `Mmu` | struct | TLB + page walk |
| `Tlb` | struct | Direct-mapped translation cache (64 entries) |
| `Perm` | enum | R / W / X — which PTE permission to check |
| `PageFault` | struct | MMU translation failure (carries vaddr) |

**7 types total.** Each has a clear counterpart in KXemu or Nemu-rust.

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
- [MEM_PLAN_REVIEW.md](./MEM_PLAN_REVIEW.md), [MEM_PLAN_REVIEW_GEMINI.md](./MEM_PLAN_REVIEW_GEMINI.md)
