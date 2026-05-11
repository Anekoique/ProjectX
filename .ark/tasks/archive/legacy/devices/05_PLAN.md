# `Device Emulation` PLAN `05`

> Status: Approved for Implementation
> Feature: `dev`
> Iteration: `05`
> Owner: Executor
> Depends on:
> - Previous Plan: `04_PLAN.md`
> - Review: `04_REVIEW.md`
> - Master Directive: `04_MASTER.md`

---

## Summary

Final plan. All blocking issues resolved in round 04. This round fixes the two remaining MEDIUM findings (TCP contract wording, QEMU-like delta list) and adds real test code per M-003.

## Log

[**Review Adjustments**]

- R-001 (TCP contract wording): Contract narrowed. `C-8`/`T-3`/`I-7` now only cover bind-failure fallback. Disconnect/reconnect moved to "future behavior" note, not part of this round's acceptance.
- R-002 (QEMU-like deltas): Added explicit delta list under C-1: "ACLINT replaces CLINT; TestFinisher is test-only."

[**Master Compliance**]

- M-001 (code quality): Pseudocode polished.
- M-002 (fix R-001/R-002): Applied as above.
- M-003 (real test code): Full Rust test code provided for all validation items.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | TCP contract narrowed to bind-fallback only; disconnect is future behavior |
| Review | R-002 | Accepted | Explicit delta list added to C-1 |
| Master | M-001 | Applied | Code polished |
| Master | M-002 | Applied | R-001/R-002 fixed |
| Master | M-003 | Applied | Real test code written below |

---

## Spec

Unchanged from `04_PLAN`. Only the following constraints are updated:

[**Constraints (updated)**]

- C-1: xemu internal layout (QEMU-like in address/size shape only). **Intentional deltas from QEMU virt:** ACLINT replaces CLINT; TestFinisher is test-only, not in default machine.
  - ACLINT: `0x0200_0000` / `0x1_0000`
  - PLIC: `0x0C00_0000` / `0x400_0000`
  - UART: `0x1000_0000` / `0x100`, IRQ = source 10
  - `timebase-frequency = 10_000_000`
- C-8: TCP: bind failure → TX-only fallback. (Disconnect/reconnect behavior is future scope, not accepted this round.)

[**Invariants (updated)**]

- I-7: TCP bind failure → UART falls back to TX-only.

All other Spec sections (Goals, Architecture, Data Structure, API Surface) are identical to `04_PLAN`.

---

## Implement

Implementation plan identical to `04_PLAN`. See `04_PLAN.md` Steps 0–5 for full pseudocode.

---

## Trade-offs

- T-3 (updated): UART TCP at `127.0.0.1:14514`. Bind failure → TX-only. This round accepts bind-fallback behavior only. Disconnect/reconnect is documented as future behavior.
- T-4 rationale note: `Device::notify()` is preferred over Bus special-casing because it keeps Bus logic generic (tick/collect/notify) without device-type knowledge. Preferred over downcast because it doesn't leak `Any` into the trait. One extra default-no-op method is the minimal cost for clean decoupling.

All other trade-offs unchanged from `04_PLAN`.

---

## Validation — Real Test Code

### ACLINT Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;
    use crate::device::Device;

    fn setup() -> (Aclint, Arc<AtomicU64>) {
        let irq = Arc::new(AtomicU64::new(0));
        (Aclint::new(irq.clone()), irq)
    }

    #[test]
    fn mtime_advances_after_tick() {
        let (mut aclint, _) = setup();
        aclint.tick();
        let t1 = aclint.read(0xBFF8, 4).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        aclint.tick();
        let t2 = aclint.read(0xBFF8, 4).unwrap();
        assert!(t2 > t1, "mtime should advance after tick");
    }

    #[test]
    fn mtime_frozen_without_tick() {
        let (mut aclint, _) = setup();
        aclint.tick();
        let t1 = aclint.read(0xBFF8, 4).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let t2 = aclint.read(0xBFF8, 4).unwrap();
        assert_eq!(t1, t2, "mtime should not advance without tick");
    }

    #[test]
    fn mtimecmp_sets_mtip() {
        let (mut aclint, irq) = setup();
        // Set mtimecmp to 0 (already expired)
        aclint.write(0x4000, 4, 0).unwrap();
        aclint.write(0x4004, 4, 0).unwrap();
        aclint.tick();
        assert_ne!(irq.load(Relaxed) & MTIP, 0, "MTIP should be set");
    }

    #[test]
    fn mtimecmp_clears_mtip() {
        let (mut aclint, irq) = setup();
        // Set mtimecmp to MAX (never expires)
        aclint.write(0x4000, 4, u32::MAX as Word).unwrap();
        aclint.write(0x4004, 4, u32::MAX as Word).unwrap();
        aclint.tick();
        assert_eq!(irq.load(Relaxed) & MTIP, 0, "MTIP should be clear");
    }

    #[test]
    fn msip_set_and_clear() {
        let (mut aclint, irq) = setup();
        aclint.write(0x0000, 4, 1).unwrap();
        assert_ne!(irq.load(Relaxed) & MSIP, 0);
        assert_eq!(aclint.read(0x0000, 4).unwrap() as u32, 1);

        aclint.write(0x0000, 4, 0).unwrap();
        assert_eq!(irq.load(Relaxed) & MSIP, 0);
        assert_eq!(aclint.read(0x0000, 4).unwrap() as u32, 0);
    }

    #[test]
    fn setssip_sets_ssip_read_returns_zero() {
        let (mut aclint, irq) = setup();
        aclint.write(0xC000, 4, 1).unwrap();
        assert_ne!(irq.load(Relaxed) & SSIP, 0, "SSIP should be set");
        assert_eq!(aclint.read(0xC000, 4).unwrap(), 0, "setssip read must return 0");
    }

    #[test]
    fn setssip_write_zero_no_effect() {
        let (mut aclint, irq) = setup();
        aclint.write(0xC000, 4, 0).unwrap();
        assert_eq!(irq.load(Relaxed) & SSIP, 0, "SSIP should not be set by write 0");
    }

    #[test]
    fn unmapped_offset_returns_zero() {
        let (mut aclint, _) = setup();
        assert_eq!(aclint.read(0x0100, 4).unwrap(), 0);
    }

    #[test]
    fn mtime_write_ignored() {
        let (mut aclint, _) = setup();
        aclint.tick();
        let before = aclint.read(0xBFF8, 4).unwrap();
        aclint.write(0xBFF8, 4, 0xDEAD).unwrap();
        assert_eq!(aclint.read(0xBFF8, 4).unwrap(), before);
    }
}
```

### PLIC Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;
    use crate::device::Device;

    fn setup() -> (Plic, Arc<AtomicU64>) {
        let irq = Arc::new(AtomicU64::new(0));
        (Plic::new(irq.clone()), irq)
    }

    #[test]
    fn priority_read_write() {
        let (mut plic, _) = setup();
        plic.write(0x04, 4, 7).unwrap(); // source 1 priority = 7
        assert_eq!(plic.read(0x04, 4).unwrap() as u8, 7);
    }

    #[test]
    fn enable_per_context() {
        let (mut plic, _) = setup();
        plic.write(0x2000, 4, 0xFF).unwrap(); // ctx 0
        plic.write(0x2080, 4, 0x0F).unwrap(); // ctx 1
        assert_eq!(plic.read(0x2000, 4).unwrap() as u32, 0xFF);
        assert_eq!(plic.read(0x2080, 4).unwrap() as u32, 0x0F);
    }

    #[test]
    fn claim_highest_priority() {
        let (mut plic, _) = setup();
        // Source 1: priority 3, source 2: priority 5
        plic.write(0x04, 4, 3).unwrap();
        plic.write(0x08, 4, 5).unwrap();
        // Enable both for ctx 0
        plic.write(0x2000, 4, 0x06).unwrap(); // bits 1,2
        // Set both pending
        plic.notify(0x06);
        // Claim should return source 2 (higher priority)
        assert_eq!(plic.read(0x200004, 4).unwrap() as u32, 2);
    }

    #[test]
    fn claim_empty_returns_zero() {
        let (mut plic, _) = setup();
        assert_eq!(plic.read(0x200004, 4).unwrap() as u32, 0);
    }

    #[test]
    fn complete_releases_claimed() {
        let (mut plic, _) = setup();
        plic.write(0x04, 4, 1).unwrap(); // src 1 prio 1
        plic.write(0x2000, 4, 0x02).unwrap(); // enable src 1 ctx 0
        plic.notify(0x02);
        let src = plic.read(0x200004, 4).unwrap() as u32;
        assert_eq!(src, 1);
        // Complete
        plic.write(0x200004, 4, 1).unwrap();
        assert_eq!(plic.claimed[0], 0);
    }

    #[test]
    fn threshold_filters() {
        let (mut plic, _) = setup();
        plic.write(0x04, 4, 3).unwrap(); // src 1 prio 3
        plic.write(0x2000, 4, 0x02).unwrap();
        plic.write(0x200000, 4, 5).unwrap(); // threshold 5 > priority 3
        plic.notify(0x02);
        assert_eq!(plic.read(0x200004, 4).unwrap() as u32, 0, "below threshold");
    }

    #[test]
    fn claimed_source_not_repended() {
        let (mut plic, _) = setup();
        plic.write(0x04, 4, 1).unwrap();
        plic.write(0x2000, 4, 0x02).unwrap();
        plic.notify(0x02);
        // Claim source 1
        plic.read(0x200004, 4).unwrap();
        // Re-notify with line still high — should NOT re-pend while claimed
        plic.notify(0x02);
        assert_eq!(plic.pending & 0x02, 0, "claimed source should not be re-pended");
    }

    #[test]
    fn source_repended_after_complete() {
        let (mut plic, _) = setup();
        plic.write(0x04, 4, 1).unwrap();
        plic.write(0x2000, 4, 0x02).unwrap();
        plic.notify(0x02);
        plic.read(0x200004, 4).unwrap(); // claim
        plic.write(0x200004, 4, 1).unwrap(); // complete
        // Line still high → next notify should re-pend
        plic.notify(0x02);
        assert_ne!(plic.pending & 0x02, 0, "should re-pend after complete");
    }

    #[test]
    fn complete_wrong_source_no_change() {
        let (mut plic, _) = setup();
        plic.write(0x04, 4, 1).unwrap();
        plic.write(0x2000, 4, 0x02).unwrap();
        plic.notify(0x02);
        plic.read(0x200004, 4).unwrap(); // claim source 1
        plic.write(0x200004, 4, 99).unwrap(); // complete wrong source
        assert_eq!(plic.claimed[0], 1, "wrong source should not release claim");
    }

    #[test]
    fn source_zero_excluded() {
        let (mut plic, _) = setup();
        plic.write(0x00, 4, 10).unwrap(); // src 0 prio 10
        plic.write(0x2000, 4, 0x01).unwrap(); // enable src 0
        plic.notify(0x01);
        assert_eq!(plic.read(0x200004, 4).unwrap() as u32, 0, "source 0 must never be claimed");
    }

    #[test]
    fn meip_seip_set_and_clear() {
        let (mut plic, irq) = setup();
        plic.write(0x04, 4, 1).unwrap(); // src 1
        plic.write(0x2000, 4, 0x02).unwrap(); // ctx 0 enable
        plic.write(0x2080, 4, 0x02).unwrap(); // ctx 1 enable
        plic.notify(0x02);
        assert_ne!(irq.load(Relaxed) & MEIP, 0);
        assert_ne!(irq.load(Relaxed) & SEIP, 0);
        // Claim + complete from both contexts
        plic.read(0x200004, 4).unwrap();
        plic.write(0x200004, 4, 1).unwrap();
        plic.read(0x201004, 4).unwrap();
        plic.write(0x201004, 4, 1).unwrap();
        // Notify with line low
        plic.notify(0x00);
        assert_eq!(irq.load(Relaxed) & MEIP, 0);
        assert_eq!(irq.load(Relaxed) & SEIP, 0);
    }
}
```

### UART Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Device;

    fn setup() -> Uart { Uart::new() }

    #[test]
    fn lsr_thre_always_set() {
        let mut uart = setup();
        let lsr = uart.read(5, 1).unwrap() as u8;
        assert_ne!(lsr & 0x60, 0, "THRE and TEMT should be set");
    }

    #[test]
    fn lsr_dr_reflects_rx_fifo() {
        let mut uart = setup();
        assert_eq!(uart.read(5, 1).unwrap() as u8 & 0x01, 0, "DR=0 when empty");
        uart.rx_fifo.push_back(0x42);
        assert_ne!(uart.read(5, 1).unwrap() as u8 & 0x01, 0, "DR=1 when data");
    }

    #[test]
    fn rbr_pops_from_fifo() {
        let mut uart = setup();
        uart.rx_fifo.push_back(0xAA);
        uart.rx_fifo.push_back(0xBB);
        assert_eq!(uart.read(0, 1).unwrap() as u8, 0xAA);
        assert_eq!(uart.read(0, 1).unwrap() as u8, 0xBB);
        assert_eq!(uart.read(0, 1).unwrap() as u8, 0x00); // empty
    }

    #[test]
    fn dlab_switches_registers() {
        let mut uart = setup();
        // Set DLAB
        uart.write(3, 1, 0x80).unwrap();
        // Write DLL/DLM
        uart.write(0, 1, 0x03).unwrap();
        uart.write(1, 1, 0x00).unwrap();
        assert_eq!(uart.read(0, 1).unwrap() as u8, 0x03); // DLL
        assert_eq!(uart.read(1, 1).unwrap() as u8, 0x00); // DLM
        // Clear DLAB
        uart.write(3, 1, 0x03).unwrap();
        // Offset 1 should now return IER
        assert_eq!(uart.read(1, 1).unwrap() as u8, 0x00); // IER
    }

    #[test]
    fn ier_write_masked() {
        let mut uart = setup();
        uart.write(1, 1, 0xFF).unwrap();
        assert_eq!(uart.read(1, 1).unwrap() as u8, 0x0F, "IER upper bits masked");
    }

    #[test]
    fn iir_rx_data_available() {
        let mut uart = setup();
        uart.ier = 0x01; // enable RX interrupt
        assert_eq!(uart.read(2, 1).unwrap() as u8, 0xC1, "no interrupt when empty");
        uart.rx_fifo.push_back(0x42);
        assert_eq!(uart.read(2, 1).unwrap() as u8, 0xC4, "RX data available");
    }

    #[test]
    fn irq_line_rx_data_and_ier() {
        let mut uart = setup();
        assert!(!uart.irq_line());
        uart.rx_fifo.push_back(0x42);
        assert!(!uart.irq_line(), "IER.rx not enabled");
        uart.ier = 0x01;
        assert!(uart.irq_line(), "should assert when data + IER");
    }

    #[test]
    fn scratch_register() {
        let mut uart = setup();
        uart.write(7, 1, 0xAB).unwrap();
        assert_eq!(uart.read(7, 1).unwrap() as u8, 0xAB);
    }

    #[test]
    fn non_byte_access_error() {
        let mut uart = setup();
        assert!(uart.read(0, 4).is_err());
        assert!(uart.write(0, 2, 0).is_err());
    }

    #[test]
    fn tick_drains_rx_buf() {
        let mut uart = setup();
        uart.rx_buf.lock().unwrap().push_back(0x11);
        uart.rx_buf.lock().unwrap().push_back(0x22);
        uart.tick();
        assert_eq!(uart.rx_fifo.len(), 2);
        assert_eq!(uart.read(0, 1).unwrap() as u8, 0x11);
    }
}
```

### TestFinisher Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Device;
    use crate::error::XError;

    #[test]
    fn pass_exit() {
        let mut tf = TestFinisher::new();
        match tf.write(0, 4, 0x5555) {
            Err(XError::ProgramExit(0)) => {}
            other => panic!("expected ProgramExit(0), got {:?}", other),
        }
    }

    #[test]
    fn fail_exit_with_code() {
        let mut tf = TestFinisher::new();
        let val = (1u32 << 16) | 0x3333;
        match tf.write(0, 4, val as Word) {
            Err(XError::ProgramExit(1)) => {}
            other => panic!("expected ProgramExit(1), got {:?}", other),
        }
    }

    #[test]
    fn read_returns_zero() {
        let mut tf = TestFinisher::new();
        assert_eq!(tf.read(0, 4).unwrap(), 0);
    }

    #[test]
    fn unknown_value_no_exit() {
        let mut tf = TestFinisher::new();
        assert!(tf.write(0, 4, 0x1234).is_ok());
    }
}
```

### Bus tick + Config Tests

```rust
#[cfg(test)]
mod tests {
    // ... existing Bus tests ...

    #[test]
    fn config_aclint_layout() {
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE);
        bus.add_mmio("aclint", 0x0200_0000, 0x1_0000,
                     Box::new(Aclint::new(Arc::new(AtomicU64::new(0)))), 0);
        let r = bus.mmio.iter().find(|r| r.name == "aclint").unwrap();
        assert_eq!(r.range.start, 0x0200_0000);
        assert_eq!(r.range.end, 0x0201_0000);
        assert_eq!(r.irq_source, 0);
    }

    #[test]
    fn config_plic_layout() {
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE);
        bus.add_mmio("plic", 0x0C00_0000, 0x400_0000,
                     Box::new(Plic::new(Arc::new(AtomicU64::new(0)))), 0);
        let r = bus.mmio.iter().find(|r| r.name == "plic").unwrap();
        assert_eq!(r.range.start, 0x0C00_0000);
        assert_eq!(r.range.end, 0x1000_0000);
    }

    #[test]
    fn config_uart_layout() {
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE);
        bus.add_mmio("uart0", 0x1000_0000, 0x100, Box::new(Uart::new()), 10);
        let r = bus.mmio.iter().find(|r| r.name == "uart0").unwrap();
        assert_eq!(r.range.start, 0x1000_0000);
        assert_eq!(r.range.end, 0x1000_0100);
        assert_eq!(r.irq_source, 10);
    }

    #[test]
    fn plic_idx_set_on_registration() {
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE);
        assert!(bus.plic_idx.is_none());
        bus.add_mmio("plic", 0x0C00_0000, 0x400_0000,
                     Box::new(Plic::new(Arc::new(AtomicU64::new(0)))), 0);
        assert!(bus.plic_idx.is_some());
    }
}
```

---

## Acceptance Mapping

Identical to `04_PLAN` with C-8 narrowed:

| Goal | Validation |
|------|------------|
| G-1 MSWI/MTIMER/SSWI | ACLINT tests above |
| G-2 PLIC | PLIC tests above |
| G-3a TX | UART tests above |
| G-3b RX | UART tick_drains_rx_buf + irq_line tests |
| G-4 Test | TestFinisher tests above |
| G-5 irq_state | Integration via Bus tick + ACLINT/PLIC irq_state assertions |
| C-1 layout | Config tests above |
| C-8 TCP | bind-failure fallback (validated by manual test; unit test uses `Uart::new()`) |
| I-3 claimed | PLIC claimed_source_not_repended + source_repended_after_complete |

---

## Files

```
xcore/src/device/
├── mod.rs            — Device trait, constants, mmio_regs!
├── bus.rs            — Bus (tick, plic_idx)
├── ram.rs
├── aclint.rs         — ACLINT (new)
├── plic.rs           — PLIC (new)
├── uart.rs           — UART 16550 (new)
└── test_finisher.rs  — TestFinisher (new, test-only)
```
