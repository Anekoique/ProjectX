[**Goals**]

- G-1: Eliminate per-instruction lock overhead by owning `Bus` inline inside `CPU` — no `Arc<Mutex<Bus>>` on the hot path.
- G-2: Use disjoint-field borrow splitting so one hart's `&mut Bus` and `&mut cores[current]` coexist without runtime locking.
- G-3: Sentinel-CI any reintroduction of `Mutex<Bus>` / `RwLock<Bus>` / `Arc<Mutex<Bus>>` anywhere under `xemu/xcore/src/`.
- G-4: Keep the lifecycle-handle `Mutex` on `XCPU` (xdb / difftest coordination) but never touch it on a per-instruction access.

[**Non-goals**]

- NG-1: No multi-thread CPU drive — true SMP belongs to a separate phase; the owned-bus design relies on the cooperative scheduler for exclusion.
- NG-2: No JIT / threaded-code dispatch — the hot-path win is about lock removal, not codegen.
- NG-3: No bus-internal locking either — devices that need concurrency hold their own atomics (PlicSignals).

[**Architecture**]

```
xemu/xcore/src/cpu/mod.rs
└── pub struct CPU<Core: CoreOps> {
        cores: Vec<Core>,
        bus:   Bus,                // <-- owned inline, NOT Arc<Mutex<Bus>>
        current: usize, ...
    }
    impl CPU::step {
        let CPU { cores, bus, current, .. } = self;     // disjoint-field split
        cores[*current].step(bus)
    }

scripts/ci/verify_no_mutex.sh
└── M-001 sentinel — regex-rejects `Mutex<Bus>` / `RwLock<Bus>` / `parking_lot::{Mutex,RwLock}<Bus>` / `Arc<Mutex<Bus>>` over the whole xcore source tree
```

The exclusion invariant is enforced by the borrow checker, not by a runtime primitive: the cooperative round-robin scheduler in `CPU::step` hands exactly one hart a `&mut Bus` per call, and Rust prevents any other code path from holding a second borrow simultaneously.

[**Data Structure**]

```rust
// Inline-owned bus inside the global CPU singleton.
pub struct CPU<Core: CoreOps> {
    cores: Vec<Core>,
    bus:   Bus,                                   // <-- inline
    current: usize,
    /* ... */
}

// Lifecycle Mutex — coordinates xdb/difftest, NOT per-access.
pub static XCPU: OnceLock<Mutex<CPU<Core>>>;
```

[**API Surface**]

```rust
// Borrow-splitting accessors for the inline bus.
impl<C: CoreOps> CPU<C> {
    pub fn bus(&self)         -> &Bus;
    pub fn bus_mut(&mut self) -> &mut Bus;
}

// Internal step uses field-disjoint destructure.
impl<C: CoreOps> CPU<C> {
    pub fn step(&mut self) -> XResult;
}
```

[**Constraints**]

- C-1: `Bus` is owned by value inside `CPU` — `xemu/xcore/src/cpu/mod.rs:103`.
- C-2: No `Mutex<Bus>` / `RwLock<Bus>` / `parking_lot::{Mutex,RwLock}<Bus>` / `Arc<Mutex<Bus>>` may appear under `xemu/xcore/src/` — enforced by `scripts/ci/verify_no_mutex.sh`.
- C-3: `CPU::step` borrows `cores` and `bus` as disjoint fields via a destructure — never via two separate `self.field` accesses inside one expression — `xemu/xcore/src/cpu/mod.rs:241`.
- C-4: The outer `Mutex<CPU>` on `XCPU` is a lifecycle handle for monitor/debugger; it is acquired once per command, not per instruction — `xemu/xcore/src/cpu/mod.rs:67`.
- C-5: A `compile_error!` sentinel in `xemu/xcore/src/device/bus.rs:38` triggers if any code adds `Mutex<Bus>` as a function parameter.
- C-6: `make test` runs `verify_no_mutex.sh` as part of the gate; the sentinel returns exit 1 on any match.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code at `xemu/xcore/src/cpu/mod.rs` + `xemu/xcore/src/device/bus.rs` + `scripts/ci/verify_no_mutex.sh`. Pre-port running notes preserved at `.ark/tasks/archive/legacy/perf-bus-fast-path/`.
