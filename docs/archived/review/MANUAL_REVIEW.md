# TODO List

1. Add per-HART support so that the behavior matches the official RISC-V documentation more closely.

2. Split ACLINT into MSWI, MTIMER, and SSWI according to the specification in the official RISC-V manuals.
   The current ACLINT implementation follows a CLINT-like design and targets only one hart, which leads to behavior that is misaligned with the official RISC-V specification.

3. The abstraction of architecture-specific behavior is poor.
   For example, the bus design seems to target only RISC-V.

4. Improve the dispatching and handling of architecture-specific behavior.
   Currently, ISA/CPU/device code dispatches architecture-related behavior through separate `riscv` and `loongarch` directories.
   This behavior seems redundant, and we may need a better design.
   Consider keeping only the highest-level abstractions in the `cpu`/`device`/`isa` directories, and moving architecture-specific behavior into an `arch` directory or module.

5. External devices(uart) should interact with PLIC directly. 
   Currently, External devices(uart) interact with PLIC with bus which is incorrect.

6. Asynchronous interrupt handle, both of external device and interrupt hanler.
   External device enable irq will notify PLIC, PLIC will handle async. And PLIC receive irq will set meip and notify Interrupt handler, interrupt handler handle async.

7. Consider Better design to PLIC, which should both support level-triggled and egde-triggled interrupt.
   Consider add the semantic of Gateways and PLIC-Core which response for different parts of PLIC.
   `device/source -> per-source gateway -> PLIC core -> hart context` Consider a good design of gateways to handle the source correctly.