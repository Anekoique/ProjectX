[**Goals**]

- G-1: Short-circuit `Mtimer::tick` when the next-fire deadline is in the future — one `u64` compare and return.
- G-2: Memoise pest decode in a per-hart direct-mapped 4096-line cache keyed on `(pc, raw)` — `ICache`.
- G-3: Aggressively `#[inline]` `checked_read` / `checked_write` / `access_bus` so the MMU fast path becomes a single function on the call graph.
- G-4: Bypass `_platform_memmove` for aligned 1 / 2 / 4 / 8-byte RAM accesses via typed `Ram::read_u{8,16,32,64}` paths.
- G-5: Split `Bus::tick` into a fast path (ACLINT every step) and a slow path (UART / PLIC every `SLOW_TICK_DIVISOR=64` steps).

[**Non-goals**]

- NG-1: No JIT / threaded-code dispatch — the bundle stays interpreted.
- NG-2: No SMP — single-hart cooperative scheduler remains; multi-hart re-profile is a separate phase.
- NG-3: No instruction-trace removal — `LOG=trace` still walks per-instruction logs.

[**Architecture**]

```
xemu/xcore/src/arch/riscv/cpu/
├── icache.rs           ICacheLine { pc, raw, decoded }; ICache [4096 lines] direct-mapped per-hart
├── mm.rs               #[inline] access_bus, checked_read, checked_write
└── mm/mmu.rs           cached SvConfig / satp_ppn / asid / sum / mxr (refreshed only on satp/mstatus writes)

xemu/xcore/src/arch/riscv/device/aclint/
└── mtimer.rs           Mtimer { next_fire_mtime: u64 } — deadline-gated tick

xemu/xcore/src/device/
├── bus.rs              tick split (fast / slow); mtime() direct accessor; LeBytes typed RAM read
└── ram.rs              read_u8 / read_u16 / read_u32 / read_u64 + write_u* typed paths
```

[**Data Structure**]

```rust
pub const ICACHE_BITS:  usize = 12;
pub const ICACHE_LINES: usize = 1 << ICACHE_BITS;     // 4096
pub const ICACHE_MASK:  usize = ICACHE_LINES - 1;

#[derive(Clone, Copy)]
pub struct ICacheLine {
    pc:      VirtAddr,
    raw:     u32,
    decoded: DecodedInst,
    valid:   bool,
}

pub struct ICache { lines: [ICacheLine; ICACHE_LINES] }
```

[**API Surface**]

```rust
impl ICache {
    pub fn new()           -> Box<Self>;
    pub fn index(pc: VirtAddr) -> usize;
    pub fn lookup(&self, pc: VirtAddr, raw: u32) -> Option<&DecodedInst>;
    pub fn insert(&mut self, pc: VirtAddr, raw: u32, decoded: DecodedInst);
}

impl Mtimer {
    pub fn next_fire(&self) -> u64;
    // tick is internal; the cached deadline short-circuit is the contract
}

impl Ram {
    pub fn read_u8 (&self, off: usize) -> u8;
    pub fn read_u16(&self, off: usize) -> u16;
    pub fn read_u32(&self, off: usize) -> u32;
    pub fn read_u64(&self, off: usize) -> u64;
    // write_u* mirror
}

impl Bus {
    pub fn mtime(&self) -> u64;                  // direct accessor, no MMIO
}
```

[**Constraints**]

- C-1: `ICache` geometry is per-hart, direct-mapped, 4096 lines — `xemu/xcore/src/arch/riscv/cpu/icache.rs:37`.
- C-2: A cache line is a hit iff `line.pc == pc && line.raw == raw` — `(pc, raw)` is the full key; decode is a pure function of `raw` and static tables — `xemu/xcore/src/arch/riscv/cpu/icache.rs`.
- C-3: Self-modifying code is handled implicitly: rewriting an instruction changes `raw`, the comparison misses, the line is overwritten — no explicit invalidation needed; `fence.i` is a NOP — `xemu/xcore/src/arch/riscv/cpu/icache.rs`.
- C-4: `Mtimer::tick` short-circuits when `bus.mtime() < self.next_fire_mtime` — no MMIO sync, no IrqState write — `xemu/xcore/src/arch/riscv/device/aclint/mtimer.rs`.
- C-5: `access_bus`, `checked_read`, `checked_write` carry `#[inline]` — `xemu/xcore/src/arch/riscv/cpu/mm.rs:254,272,284`.
- C-6: Aligned 1 / 2 / 4 / 8-byte RAM accesses use `Ram::read_u{8,16,32,64}` and bypass `_platform_memmove` — `xemu/xcore/src/device/ram.rs`.
- C-7: `Bus::tick` runs ACLINT every step and UART / PLIC every `SLOW_TICK_DIVISOR` steps — `xemu/xcore/src/device/bus.rs:267`.
- C-8: `Mmu` caches `satp_ppn` / `asid` / `sv_config` / `sum` / `mxr`; refresh only on `update_satp` / `update_mstatus` — `xemu/xcore/src/arch/riscv/cpu/mm/mmu.rs:43`.
- C-9: PMP M-mode fast path skips the 16-entry scan when no entry has the `L` bit — `xemu/xcore/src/arch/riscv/cpu/mm/pmp.rs:171`.
- C-10: Cumulative user-time reduction vs the pre-P1 2026-04-14 baseline: dhrystone −57 %, coremark −58 %, microbench −62 % — baseline data at `docs/perf/baselines/2026-04-16/`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xemu/xcore/src/arch/riscv/cpu/icache.rs`, `xemu/xcore/src/arch/riscv/cpu/mm*`, `xemu/xcore/src/device/{bus,ram}.rs`. Pre-port running notes preserved at `.ark/tasks/archive/legacy/perf-hot-path/`.
