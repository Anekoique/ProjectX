[**Goals**]

- G-1: Provide WARL-masked read / write for every M / S / U CSR xemu emulates (mstatus, sstatus, mip, sie, satp, ...).
- G-2: Route every architectural trap — ecall, ebreak, illegal-inst, page-fault — through `mtvec` / `stvec` via `medeleg` / `mideleg`.
- G-3: Shadow S-mode CSRs onto M-mode storage via one descriptor table — no duplicate state for `sstatus` / `sip` / `sie`.
- G-4: Make architectural traps Err-driven: handlers emit `Err(XError::Trap)`, `trap_on_err` drains it into `commit_trap`.
- G-5: Auto-generate the difftest CSR whitelist from `csr_table!` `@ difftest` annotations.

[**Non-goals**]

- NG-1: No HPM / hpmcounter / hpmevent CSRs beyond write-through stubs.
- NG-2: No vector-extension CSRs (`vstart`, `vl`, `vtype`) — out of scope for the current ISA set.
- NG-3: No vectored `mtvec` mode (BASE+4×cause); `mtvec.MODE` wmask forces direct mode.

[**Architecture**]

```
xemu/xcore/src/arch/riscv/cpu/
├── csr.rs              CsrFile, CsrDesc, AccessRule, csr_table!, find_desc, DIFFTEST_CSRS
├── csr/
│   ├── mip.rs          bitflags! Mip
│   ├── mstatus.rs      bitflags! MStatus + mpp/with_mpp/spp/with_spp helpers
│   ├── ops.rs          impl RVCore { csr read/write entry points }
│   └── privilege.rs    PrivilegeMode { M, S, U } + from_bits
├── trap.rs             impl RVCore { trap, trap_exception, illegal_inst, trap_on_err }
└── trap/
    ├── cause.rs        PendingTrap + TrapCause
    ├── exception.rs    Exception enum
    ├── handler.rs      impl RVCore { check_pending_interrupts, commit_trap, do_mret, do_sret }
    └── interrupt.rs    Interrupt enum + SSIP / MSIP / STIP / MTIP / SEIP / MEIP / HW_IP_MASK
```

Trap pipe (consolidates the former `err2trap` refactor): architectural traps emit `Err(XError::Trap(PendingTrap{cause, tval}))`; `trap_on_err` is the single drain site that lifts the Err into `commit_trap`. `CsrFile` is dumb storage + WARL masks; privilege checks, dynamic rules (TSR / TVM / counteren), side effects, and trap generation live in `RVCore`.

[**Data Structure**]

```rust
pub struct CsrFile { /* private storage; access via methods */ }

pub struct CsrDesc {
    pub wmask:      Word,    // writable bits (WARL)
    pub storage:    u16,     // backing CSR addr (may differ for aliases / shadows)
    pub view_mask:  Word,    // visible subfield within storage
    pub view_shift: u8,      // right-shift applied to storage before masking (frm-style)
    pub access:     AccessRule,
}

pub enum AccessRule {
    Standard,
    BlockedByMstatus(MStatus),
    CounterGated,
    RequireFP,
}

pub enum PrivilegeMode { M, S, U }

pub struct PendingTrap { pub cause: TrapCause, pub tval: Word }
pub enum TrapCause { Exception(Exception), Interrupt(Interrupt) }

bitflags! { pub struct MStatus: Word { /* SIE / MIE / SPIE / MPIE / SPP / MPP / MPRV / SUM / MXR / TVM / TW / TSR / FS / XS / SD */ } }
bitflags! { pub struct Mip:     Word { /* SSIP / MSIP / STIP / MTIP / SEIP / MEIP */ } }
```

[**API Surface**]

```rust
impl CsrFile {
    pub fn new() -> Self;
    pub fn get        (&self, addr: CsrAddr)               -> Word;
    pub fn get_by_addr(&self, addr: u16)                   -> Word;
    pub fn set        (&mut self, addr: CsrAddr, val: Word);
    pub fn read_with_desc (&self,     desc: CsrDesc)            -> Word;
    pub fn write_with_desc(&mut self, desc: CsrDesc, val: Word);
    pub fn read_masked    (&self,     addr: u16)                -> Option<Word>;
    pub fn write_masked   (&mut self, addr: u16,   val: Word)   -> bool;
    pub fn increment_cycle  (&mut self);
    pub fn increment_instret(&mut self);
}

impl RVCore {
    // Trap pipe (crate-internal — only RVCore call sites use these directly)
    fn trap          (&self, cause: TrapCause, tval: Word) -> XError;
    fn trap_exception(&self, exc:   Exception, tval: Word) -> XError;
    fn illegal_inst  (&self)                               -> XError;
    fn trap_on_err   (&mut self, bus: &mut Bus,
                      f: impl FnOnce(&mut Self, &mut Bus) -> XResult) -> XResult;
    // Trap commit (public)
    pub fn check_pending_interrupts(&mut self) -> bool;
    pub fn commit_trap(&mut self, trap: PendingTrap);
    pub fn do_mret    (&mut self);
    pub fn do_sret    (&mut self);
}

macro_rules! csr_table { /* emits CsrAddr enum + find_desc + DIFFTEST_CSRS */ }
pub(in crate::arch::riscv) fn find_desc(addr: u16) -> Option<CsrDesc>;
pub const DIFFTEST_CSRS: &[(CsrAddr, u64)];
```

[**Constraints**]

- C-1: `mstatus` is the master; `sstatus` is a subset view over `mstatus` storage — `xemu/xcore/src/arch/riscv/cpu/csr/mstatus.rs`.
- C-2: Architectural traps emit `Err(XError::Trap(PendingTrap{cause, tval}))`; `trap_on_err` drains them into `commit_trap` — `xemu/xcore/src/arch/riscv/cpu/trap.rs`.
- C-3: `commit_trap` is the single PC + CSR commit point for traps — `xemu/xcore/src/arch/riscv/cpu/trap/handler.rs:60`.
- C-4: `CsrFile` is storage + WARL masking; privilege, dynamic rules, side effects, and trap generation live in `RVCore` — `xemu/xcore/src/arch/riscv/cpu/csr.rs`.
- C-5: `csr_table!` is the single source of `CsrAddr` and `find_desc` — `xemu/xcore/src/arch/riscv/cpu/csr.rs:66`.
- C-6: WARL masking applies on every write — `write_with_desc` ANDs `view_mask & wmask` — `xemu/xcore/src/arch/riscv/cpu/csr.rs:316`.
- C-7: `mret` and `sret` restore privilege from `mstatus.MPP` / `mstatus.SPP` and clear `MPRV` per spec — `xemu/xcore/src/arch/riscv/cpu/trap/handler.rs:129`.
- C-8: Illegal CSR access raises `Exception::IllegalInstruction`; never returns `Err(XError)` for guest-visible illegal — `xemu/xcore/src/arch/riscv/cpu/csr/ops.rs`.
- C-9: `Err(XError)` variants other than `Trap` are reserved for host I/O failures and emulator invariant violations — `xemu/xcore/src/error.rs`.
- C-10: `csr_table!` entries tagged `@ difftest` are auto-collected into `DIFFTEST_CSRS`; manual edits to the whitelist are forbidden.
- C-11: Interrupt-pending bits `MSIP` / `MTIP` / `SEIP` / `MEIP` are HW-only writes — guest `csrw mip` is masked off — `xemu/xcore/src/arch/riscv/cpu/trap/interrupt.rs:22` (`HW_IP_MASK`).

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xemu/xcore/src/arch/riscv/cpu/csr*` and `cpu/trap*`. Absorbs the legacy `err2trap` refactor (single trap pipe). Pre-port running-notes preserved at `.ark/tasks/archive/legacy/csr/SPEC_LEGACY.md` and `.ark/tasks/archive/legacy/err2trap/SPEC_LEGACY.md`.
