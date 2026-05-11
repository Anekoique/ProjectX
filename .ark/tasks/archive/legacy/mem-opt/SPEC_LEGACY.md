# `memOpt` SPEC

> Memory subsystem optimization (follow-up to `mm`).
>
> **Source:** [`/docs/archived/feat/mm/MEM_OPTIMIZATION_PLAN.md`](/docs/archived/feat/mm/MEM_OPTIMIZATION_PLAN.md) — pre-workflow design document,
> preserved verbatim as the authoritative spec for this feature.
> The layout does not match `docs/template/SPEC.template`; rewrite
> to the template shape when the feature next sees meaningful
> iteration.

---

# Memory Subsystem Optimization Plan

> Post-implementation optimization for Phase 3 memory management.
> Goal: reduce performance regression from memory subsystem, improve code quality.

## Performance Analysis

### Current hot path: `load(vaddr, size)` → 4 lock acquisitions

```
load(vaddr, size)
  → translate(vaddr, size, op)
    → mmu_translate(vaddr, op)
      → bus.lock() ← LOCK 1 (for page walk / TLB)
      → mmu.translate(... &bus)  // page walk reads PTE via bus.read_ram()
      → drop(bus)
    → pmp.check(paddr, size, op, priv)
  → bus_read(paddr, size, ...)
    → bus.lock() ← LOCK 2 (for actual data read)
    → bus.read(paddr, size)
    → drop(bus)
```

For a 32-bit instruction fetch with cross-page: **4 locks** (2 parcels × 2 locks each).
For page walk with 3-level TLB miss: **1 lock** holding 3 `read_ram` calls + **1 lock** for data.

`std::sync::Mutex` uncontested lock/unlock: ~25-40ns each. At 4 locks per load:
**100-160ns overhead per memory access just from locking.**

### Root cause: `Arc<Mutex<Bus>>` design

The bus is behind a mutex because we planned for multi-core. But the emulator
is currently single-core. The mutex is uncontested 100% of the time — pure overhead.

## Optimization Plan (prioritized)

### O1. [HIGH IMPACT] Eliminate double-lock: merge translate + bus access

**Current**: `mmu_translate` locks bus → drops → `bus_read` locks again.
**Fix**: Hold one lock across translate + bus access.

```rust
pub(super) fn load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
    if !addr.is_aligned(size) { return self.trap_exception(...) }
    let mut bus = self.bus.lock().unwrap();
    let priv_mode = self.effective_priv();
    let paddr = self.mmu.translate(addr, MemOp::Load, priv_mode, &self.pmp, &bus)
        .map_err(|e| Self::to_trap(e, addr, MemOp::Load))?;
    self.pmp.check(paddr, size, MemOp::Load, priv_mode)
        .map_err(|e| Self::to_trap(e, addr, MemOp::Load))?;
    bus.read(paddr, size)
        .map_err(|e| Self::to_trap(e, addr, MemOp::Load))
}
```

**Impact**: 2 locks → 1 lock per access. **50% reduction in lock overhead.**
**Complexity**: Low. Restructure `mem.rs` only.

### O2. [HIGH IMPACT] TLB fast path bypasses bus lock entirely

When TLB hits (expected 80-95% of accesses), no page walk is needed. The
translate can be done without holding the bus lock:

```rust
pub(super) fn load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
    if !addr.is_aligned(size) { return self.trap_exception(...) }
    let priv_mode = self.effective_priv();

    // TLB fast path: no bus lock needed
    let paddr = if let Some(cached) = self.mmu.tlb_lookup(addr, MemOp::Load, priv_mode) {
        cached
    } else {
        // TLB miss: lock bus for page walk
        let bus = self.bus.lock().unwrap();
        self.mmu.translate_slow(addr, MemOp::Load, priv_mode, &self.pmp, &bus)?
    };

    self.pmp.check(paddr, size, MemOp::Load, priv_mode)?;
    self.bus.lock().unwrap().read(paddr, size)
        .map_err(|e| Self::to_trap(e, addr, MemOp::Load))
}
```

**Impact**: TLB hit = 1 lock (bus read only). TLB miss = 2 locks.
With 90% TLB hit rate: average 1.1 locks per access vs current 2.
**Complexity**: Medium. Split `Mmu::translate` into fast/slow paths.

### O3. [HIGH IMPACT] Replace `std::sync::Mutex` with `parking_lot::Mutex`

`parking_lot::Mutex` is 1.5x faster for uncontested locks (our case).
Drop-in replacement — same API, just change the import.

```toml
# Cargo.toml
parking_lot = "0.12"
```

```rust
use parking_lot::Mutex;  // replaces std::sync::Mutex
// MutexGuard API is identical
```

**Impact**: ~30-40% reduction in lock/unlock time.
**Complexity**: Trivial. Change import + Cargo.toml.

### O4. [MEDIUM IMPACT] Inline hot-path functions

Functions called on every memory access should be `#[inline]`:

- `Pte::flags()`, `is_valid()`, `is_leaf()`, `ppn()`
- `TlbEntry::matches()`, `permits()`, `translate()`
- `Tlb::get()`
- `Pmp::check()` (called twice per access)
- `RVCore::effective_priv()`

**Impact**: Eliminates function call overhead on hot path (~5-10ns per call).
**Complexity**: Low. Add `#[inline]` attributes.

### O5. [MEDIUM IMPACT] Reduce `MemOp` matching overhead

`to_trap` does a 2-level match on every error. Since errors are rare, this
is fine. But `Pmp::check` and `TlbEntry::permits` match on `MemOp` per access.
Consider bit-packing MemOp into a single permission bit mask:

```rust
impl MemOp {
    fn perm_mask(self) -> u8 {
        match self {
            MemOp::Fetch => 0x4, // X bit
            MemOp::Load  => 0x1, // R bit
            MemOp::Store | MemOp::Amo => 0x2, // W bit
        }
    }
}

// In PmpEntry::permits:
fn permits(self, op: MemOp) -> bool {
    self.cfg & op.perm_mask() != 0
}
```

**Impact**: Replaces match with single AND. Small but measurable on hot path.
**Complexity**: Low.

### O6. [LOW IMPACT] Consider `Rc<RefCell<Bus>>` for single-core

If multi-core is not imminent, `Rc<RefCell<Bus>>` eliminates all lock overhead:
- `RefCell::borrow()`: ~2ns (vs Mutex: ~25ns)
- `RefCell::borrow_mut()`: ~2ns

**Impact**: 10x faster bus access. But loses `Send + Sync` — cannot go multi-core.
**Risk**: Blocks future multi-core without refactoring back to Mutex.
**Decision**: Defer unless benchmarks show Mutex is the dominant bottleneck.

### O7. [LOW IMPACT] Code quality: reduce `mem.rs` duplication

`load`/`store`/`amo_load`/`amo_store` are near-identical. Extract:

```rust
fn mem_access(&mut self, addr: VirtAddr, size: usize, op: MemOp,
              access: impl FnOnce(&mut Bus, usize, usize) -> XResult<Word>) -> XResult<Word> {
    // alignment, translate, pmp, bus access — all in one
}
```

**Impact**: Code size reduction, easier maintenance. No performance change.
**Complexity**: Low.

## Recommended Implementation Order

1. **O1** (merge locks) — biggest win, lowest risk
2. **O3** (`parking_lot`) — trivial change, measurable win
3. **O4** (inline hot paths) — easy, small win
4. **O5** (MemOp perm_mask) — easy, small win
5. **O2** (TLB fast path) — medium effort, big win for paging workloads
6. **O7** (code dedup) — quality improvement
7. **O6** (RefCell) — only if benchmarks demand it

## Expected Impact

| Optimization | Lock reduction | Time saved/access | Complexity |
|-------------|---------------|-------------------|------------|
| O1 merge locks | 2 → 1 | ~30ns | Low |
| O2 TLB fast path | 2 → 1.1 avg | ~25ns avg | Medium |
| O3 parking_lot | same count, faster | ~10ns | Trivial |
| O4 inline | — | ~5-10ns | Low |
| O5 perm_mask | — | ~2ns | Low |
| **Combined O1+O3+O4** | **2 → 1, faster** | **~45ns** | **Low** |

Current estimated overhead: ~100ns per memory access (4 locks × 25ns).
After O1+O3+O4: ~25ns (1 lock × 15ns + inline savings).
**~4x improvement on memory access hot path.**

## References

- parking_lot benchmark: 1.5x faster uncontested, 5x contended
- RefCell vs Mutex: 10x faster single-threaded
- TLB hit rates: 80-95% in typical OS workloads
- Production emulators (QEMU, Spike): all use soft-TLB before page walk
