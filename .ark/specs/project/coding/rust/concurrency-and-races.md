# Concurrency and Races

> Rules for lock ordering, atomic operations, and race-free critical sections. Concurrency code is reviewed with extreme rigor; the rules below are correctness-bearing.

## R1 — Establish and document a consistent lock-acquisition order

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Acquiring two locks in different orders from different code paths is a deadlock waiting to happen. Document the hierarchical lock order at the call site (a comment on the function, or a comment immediately before the first acquisition).

```rust
// Bad — silent acquisition order
pub(super) fn set_control(self: Arc<Self>, process: &Process) -> Result<()> {
    let process_group_mut = process.process_group.lock();
    let session_inner = self.session.inner.lock();
    let job_control = self.job_control.lock();
    ...
}

// Good
pub(super) fn set_control(self: Arc<Self>, process: &Process) -> Result<()> {
    // Lock order: process_group -> session.inner -> job_control
    let process_group_mut = process.process_group.lock();
    let session_inner = self.session.inner.lock();
    let job_control = self.job_control.lock();
    ...
}
```

## R2 — Never perform I/O or other blocking operations while holding a spinlock

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Holding a spinlock across I/O or any blocking call is a deadlock hazard — the holding CPU stalls every other waiter. Drop the lock first or use a sleeping mutex.

```rust
// Bad — I/O under spinlock
let guard = self.state.lock();        // SpinLock<...>
self.device.write(&guard.pending_data)?;

// Good — release the lock before I/O
let data = {
    let guard = self.state.lock();
    guard.pending_data.clone()
};
self.device.write(&data)?;
```

## R3 — Correlated state requires a single lock, not multiple atomics

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Two atomic fields that must be observed in concert can be read in an inconsistent intermediate state. Wrap them in a lock instead. Use atomics only when a single value is genuinely independent.

```rust
// Bad — observers can see inconsistent state
struct Stats {
    total_bytes: AtomicU64,
    total_packets: AtomicU64,
}

// Good — lock protects the correlated pair
struct Stats {
    inner: SpinLock<StatsInner>,
}
struct StatsInner {
    total_bytes: u64,
    total_packets: u64,
}
```

## R4 — Check-then-act sequences happen under one lock acquisition

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Operations where a check determines a conditional action must take both check and action under the same lock. Releasing between the two opens a TOCTOU race.

```rust
// Bad — TOCTOU: state can change between the two acquisitions
let is_ready = self.inner.lock().state == State::Ready;
if is_ready {
    self.inner.lock().state = State::Running;
    self.inner.lock().start();
}

// Good — single critical section
let mut inner = self.inner.lock();
if inner.state == State::Ready {
    inner.state = State::Running;
    inner.start();
}
```
