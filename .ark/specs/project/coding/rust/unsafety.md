# Unsafety

> Rules for `unsafe` Rust across the workspace. Astervisor's TCB-size claim depends on these — every `unsafe` block is part of the trusted computing base, so each one needs a documented reason and a bounded audit surface.

## R1 — Every `unsafe` block is preceded by a `// SAFETY:` comment justifying soundness

**Applies to:** `**/*.rs`
**Evidence:** `clippy::undocumented_unsafe_blocks`

State the invariants that make this specific operation sound: what the inputs guarantee, what state the surrounding code holds, why aliasing or lifetime rules are not violated. For multi-condition justifications, use a numbered list so each invariant is independently auditable.

```rust
// Bad — bare unsafe, no justification
unsafe {
    context_switch(next_task_ctx_ptr, current_task_ctx_ptr);
}

// Good
// SAFETY:
// 1. We have exclusive access to both the current context and the next
//    context (both produced by `Task::new` and held in this scheduler).
// 2. The next context is valid because it was either correctly initialised
//    on Task creation or written by a previous `context_switch`.
unsafe {
    context_switch(next_task_ctx_ptr, current_task_ctx_ptr);
}
```

## R2 — Every `unsafe fn` and `unsafe trait` declares a `# Safety` rustdoc section

**Applies to:** `**/*.rs`
**Evidence:** `clippy::missing_safety_doc`

The `# Safety` section states what the *caller* must guarantee for the call to be sound. Do not describe what the function does internally — that is the role of the rest of the doc comment.

```rust
// Bad — no Safety section
/// Switches to the next task context.
pub unsafe fn context_switch(next: *mut Ctx, curr: *mut Ctx) { ... }

// Good
/// Switches to the next task context.
///
/// # Safety
///
/// Both `next` and `curr` must point to valid, exclusively-owned `Ctx`
/// values, and the caller must hold the scheduler lock.
pub unsafe fn context_switch(next: *mut Ctx, curr: *mut Ctx) { ... }

// Good — unsafe trait
/// A marker trait for guard types that enforce the atomic mode.
///
/// # Safety
///
/// The implementer must ensure that the atomic mode is maintained while
/// the guard type is alive.
pub unsafe trait InAtomicMode: core::fmt::Debug {}
```

## R3 — Crates under `visor/` set `#![deny(unsafe_code)]`; only `ostd/` may contain `unsafe`

**Applies to:** `visor/**/lib.rs`, `visor/**/main.rs`
**Evidence:** grep

This is the TCB boundary the project's language-isolation premise rests on. Every `unsafe` operation `visor/` needs is exposed as a safe API by `ostd/`. If a `visor/` crate genuinely needs an unsafe operation OSTD does not provide, the right move is to add a safe wrapper in `ostd/`, not to relax the deny — see `docs/ROADMAP.md` § Component Allocation and `AGENTS.md`.

```rust
// Bad — visor crate without the deny
// in visor/src/lib.rs:
#![no_std]
#![no_main]

// Good
// in visor/src/lib.rs:
#![no_std]
#![no_main]
#![deny(unsafe_code)]
```

## R4 — `unsafe` lives in the smallest module that owns the relied-upon state

**Applies to:** `ostd/**/*.rs`
**Evidence:** VERIFY

The safety argument extends to *every* item in the same module that can mutate the invariant-bearing state — those items are part of the audit surface. Encapsulate the unsafe abstraction in the smallest possible module so the surface stays small and reviewable.

```rust
// Bad — invariant on `next` exposed across the whole crate
pub struct FrameAlloc {
    pub next: usize,    // anyone in the crate can break this
}
impl FrameAlloc {
    pub fn alloc(&mut self) -> PhysAddr {
        // SAFETY: `next` is always valid... but we cannot prove that
        // from inside this method, because callers can mutate `next`.
        unsafe { self.alloc_frame_unchecked(self.next) }
    }
}

// Good — invariant scoped to a private module
mod frame_allocator {
    /// Invariant: `next` is always a valid frame index.
    pub struct FrameAlloc {
        next: usize,    // private; only this module can mutate
    }
    impl FrameAlloc {
        pub fn alloc(&mut self) -> PhysAddr {
            // SAFETY: `next` is always valid (see invariant above);
            // only code in this module can modify `next`.
            unsafe { self.alloc_frame_unchecked(self.next) }
        }
    }
}
```
