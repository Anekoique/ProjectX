# Performance

> Rules for performance-bearing changes. The Phase-4 evaluation argument depends on hot paths actually being fast — but premature optimization without evidence is also rejected.

## R1 — Hot paths do not introduce O(n) algorithms in n that grows with workload

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

System call dispatch, scheduler enqueue, channel send, page-fault handling — paths that fire on every event must not have O(n) complexity in any quantity that scales with the workload (number of processes, file descriptors, channels). Use a balanced tree, a heap, or a hashmap.

```rust
// Bad — O(n) scan on every enqueue
fn select_cpu(&self, cpus: &[CpuState]) -> CpuId {
    cpus.iter()
        .min_by_key(|c| c.load())
        .expect("at least one CPU")
        .id()
}

// Good — O(log n) heap peek
fn select_cpu(&self) -> CpuId {
    self.cpu_heap.peek().expect("at least one CPU").id()
}
```

## R2 — Avoid copies and allocations that the type system does not require

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Cloning an `Arc` when a `&` reference suffices, collecting an iterator into a `Vec` when the consumer is also an iterator, serializing into a stack buffer before writing — all are correctness-preserving but cost cycles. Take borrows by default; widen to ownership only when the callee needs to retain the value.

```rust
// Bad — Arc::clone where a borrow suffices
fn process(&self, stream: Arc<DmaStream>) {
    let s = stream.clone();
    s.sync();
}

// Good — borrow when ownership is not needed
fn process(&self, stream: &DmaStream) {
    stream.sync();
}
```

## R3 — Performance optimizations are justified with measurements

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

A change motivated by performance must show before/after numbers in the commit message or the linked PR. "I think this is faster" is not a justification — complexity added to solve a non-existent problem is a defect.

```rust
// Bad — added inline assembly with no benchmark
#[inline(always)]
unsafe fn fast_memcpy(...) {
    asm!(...);   // claimed faster than core::ptr::copy_nonoverlapping
}

// Good — change accompanied by numbers (in the commit body):
//
//   perf(mm): vectorize page-zero loop
//
//   Benchmark: zero 1 MiB page on x86_64 (avg of 100 runs)
//     Before: 47.2 µs
//     After:  18.6 µs   (-60.6%)
```
