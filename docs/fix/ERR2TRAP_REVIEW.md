# Review of ERR2TRAP: The "TrapTaken" Approach vs "Internal ExecFlow"

The improved plan presented in `ERR2TRAP.md` takes a much better architectural direction. By avoiding scattered `match` statements and creating a set of centralized façades (`fetch_inst`, `decode_inst`, `execute_inst`, `mem_read_load`, etc.), the codebase remains clean and instruction handlers can continue to use `?` seamlessly.

Here is a constructive review of the proposed "TrapTaken" approach and an alternative idea that achieves the exact same architectural goals but without polluting the public `XError` enum.

## 1. Critique of adding `TrapTaken` to `XError`

**The Proposal:**
Add `TrapTaken` to `pub enum XError` so it can be returned inside `XResult`.

**The Critique:**
While this is technically the path of least resistance (fewest lines of code changed), it mixes domain layers. `XError` is part of the public API of the `xcore` crate. Adding a purely internal control-flow signal (`TrapTaken`) to a public error enum means:
1. Every consumer of `xcore` (like `xdb` or the future OS loop) now sees `TrapTaken` as a possible error state and has to decide what to do with it, even though it's technically illegal for it to leak past `CPU::step()`.
2. It breaks the semantic meaning of `XResult`. A trap isn't an emulator failure; it's a valid architectural state.

## 2. A Better Idea: The `StepResult` / `TrapTaken` Separation

Instead of modifying the globally used `XError` and `XResult`, we can achieve the exact same façade benefits by introducing a private control flow enum **only for the execution loop**.

Keep `XError` and `XResult` exactly as they are today:
```rust
pub enum XError {
    BadAddress,
    AddrNotAligned,
    InvalidInst,
    // NO TrapTaken here!
}
pub type XResult<T = ()> = Result<T, XError>;
```

### Introduce `ExecResult` internally within `cpu/riscv/mod.rs`

Define a new type just for the execution pipeline that separates Host Errors (`XError`) from Guest Traps:

```rust
pub(crate) enum ExecFlow {
    TrapTaken,
    HostError(XError),
}

// Implement From<XError> so `?` works seamlessly for host errors
impl From<XError> for ExecFlow {
    fn from(err: XError) -> Self {
        ExecFlow::HostError(err)
    }
}

pub(crate) type ExecResult<T = ()> = Result<T, ExecFlow>;
```

### The helper becomes:

```rust
impl RVCore {
    #[inline(always)]
    fn take_trap<T>(&mut self, cause: TrapCause, tval: Word) -> ExecResult<T> {
        self.raise_trap(cause, tval);
        Err(ExecFlow::TrapTaken)
    }
}
```

### The Façades (The boundaries)

The instruction handlers (in `inst/*.rs`) will still return `XResult` and use `?`. The façades are the ones that convert `XResult` into `ExecResult`.

```rust
impl RVCore {
    // 1. Fetch façade
    fn fetch_inst(&mut self) -> ExecResult<u32> {
        let word = match with_mem!(fetch_u32(self.virt_to_phys(self.pc), 4)) {
            Ok(word) => word,
            Err(err) => return self.trap_mem(err, MemTrapKind::Fetch, self.pc.as_usize() as Word), // returns ExecResult
        };
        // ...
    }

    // 2. Decode façade
    fn decode_inst(&mut self, raw: u32) -> ExecResult<DecodedInst> {
        match self.decode(raw) { // self.decode returns XResult
            Ok(inst) => Ok(inst),
            Err(XError::InvalidInst) => self.take_trap(TrapCause::Exception(Exception::IllegalInstruction), raw as Word),
            Err(err) => Err(err.into()), // auto-converts to ExecFlow::HostError
        }
    }

    // 3. Execute façade
    fn execute_inst(&mut self, raw: u32, inst: DecodedInst) -> ExecResult {
        // self.dispatch returns XResult
        match self.dispatch(inst) {
            Ok(()) => Ok(()),
            Err(XError::InvalidInst) => self.take_trap(TrapCause::Exception(Exception::IllegalInstruction), raw as Word),
            Err(err) => Err(err.into()),
        }
    }
}
```

*(You would do the same for `mem_read_load` and `mem_write_store` as described in your plan, but they would return `ExecResult`)*

### The `step()` function cleanly resolves it

Now `step()` remains `-> XResult` (public API), but internally consumes `ExecFlow`:

```rust
impl CoreOps for RVCore {
    fn step(&mut self) -> XResult {
        if self.check_pending_interrupts() {
            self.retire();
            return Ok(());
        }

        let result = (|| -> ExecResult {
            let raw = self.fetch_inst()?;
            let inst = self.decode_inst(raw)?;
            self.execute_inst(raw, inst)
        })();

        match result {
            Ok(()) | Err(ExecFlow::TrapTaken) => {
                self.retire();
                Ok(())
            }
            Err(ExecFlow::HostError(err)) => Err(err), // Propagate actual host errors
        }
    }
}
```

## Summary of the Design

1.  **Instruction Handlers (`inst/*.rs`) stay exactly the same.** They return `XResult` and use `?`. They don't know about `TrapTaken` or `ExecFlow`.
2.  **`XError` remains pure.** We don't pollute the global error enum with control-flow logic.
3.  **The Boundaries (Façades) do the heavy lifting.** They map guest-visible `XError`s into `take_trap` (which returns `ExecFlow::TrapTaken`), and bubble up true internal errors as `ExecFlow::HostError(err)`.
4.  **`step()` unwraps the execution flow.** It commits traps and safely halts on real emulator bugs.

This approach gives you the exact same runtime behavior and architectural cleanliness as your proposed plan, but without leaking `TrapTaken` into the public `XError` type. It strictly bounds the "control flow as error" pattern to the execution pipeline where it belongs.