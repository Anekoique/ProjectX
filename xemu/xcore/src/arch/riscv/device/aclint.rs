//! ACLINT: RISC-V Advanced Core Local Interruptor.
//!
//! Composed of three spec-mandated sub-devices (MSWI + MTIMER + SSWI),
//! each mounted as its own MMIO region by [`Aclint::install`].

mod mswi;
mod mtimer;
mod sswi;

use self::{mswi::Mswi, mtimer::Mtimer, sswi::Sswi};
use crate::device::{IrqState, bus::Bus};

/// ACLINT façade: composes MSWI + MTIMER + SSWI and installs them on the bus.
pub struct Aclint {
    mswi: Mswi,
    mtimer: Mtimer,
    sswi: Sswi,
}

impl Aclint {
    /// Build all three sub-devices with per-hart IRQ state. SSIP edge
    /// signals route through `IrqState::raise_ssip_edge` (directIrq Task 5).
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self {
        debug_assert_eq!(irqs.len(), num_harts);
        Self {
            mswi: Mswi::new(num_harts, irqs.clone()),
            mtimer: Mtimer::new(num_harts, irqs.clone()),
            sswi: Sswi::new(num_harts, irqs),
        }
    }

    /// Register the three sub-devices on `bus` at the given `base`:
    ///   MSWI   at base+0x0000 (size 0x4000)
    ///   MTIMER at base+0x4000 (size 0x8000)
    ///   SSWI   at base+0xC000 (size 0x4000)
    /// Returns the bus index of the MTIMER region (for
    /// `Bus::set_timer_source`).
    pub fn install(self, bus: &mut Bus, base: usize) -> usize {
        bus.add_mmio("mswi", base, 0x4000, Box::new(self.mswi));
        let mtimer_idx = bus.add_mmio("mtimer", base + 0x4000, 0x8000, Box::new(self.mtimer));
        bus.add_mmio("sswi", base + 0xC000, 0x4000, Box::new(self.sswi));
        mtimer_idx
    }
}

#[cfg(test)]
pub(super) mod test_utils {
    use crate::device::IrqState;

    /// Construct `n` independent per-hart IRQ states.
    pub fn make_irqs(n: usize) -> Vec<IrqState> {
        (0..n).map(|_| IrqState::new()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        arch::riscv::cpu::trap::interrupt::{MSIP, MTIP},
        config::{CONFIG_MBASE, CONFIG_MSIZE, Word},
        device::bus::Bus,
    };

    const ACLINT_BASE: usize = 0x0200_0000;

    fn new_bus() -> Bus {
        Bus::new(CONFIG_MBASE, CONFIG_MSIZE, 1)
    }

    fn new_aclint() -> (Aclint, IrqState) {
        let irq = IrqState::new();
        (Aclint::new(1, vec![irq.clone()]), irq)
    }

    #[test]
    fn install_wires_all_three() {
        let mut bus = new_bus();
        let (aclint, irq) = new_aclint();
        let mtimer_idx = aclint.install(&mut bus, ACLINT_BASE);
        bus.set_timer_source(mtimer_idx);

        // MSWI — MSIP toggles MSIP bit at global +0x0000.
        bus.write(ACLINT_BASE + 0x0000, 4, 1).unwrap();
        assert_ne!(irq.load() & MSIP, 0);
        assert_eq!(bus.read(ACLINT_BASE + 0x0000, 4).unwrap() as u32, 1);

        // MTIMER — mtimecmp=0 (global +0x4000/+0x4004) → MTIP set on tick.
        bus.write(ACLINT_BASE + 0x4000, 4, 0).unwrap();
        bus.write(ACLINT_BASE + 0x4004, 4, 0).unwrap();
        bus.tick();
        assert_ne!(irq.load() & MTIP, 0);
        // mtime is readable at global +0xBFF8/+0xBFFC.
        let _ = bus.read(ACLINT_BASE + 0xBFF8, 4).unwrap();
        let _ = bus.read(ACLINT_BASE + 0xBFFC, 4).unwrap();

        // SSWI — edge-triggered pending flag at global +0xC000.
        bus.write(ACLINT_BASE + 0xC000, 4, 1).unwrap();
        assert!(irq.take_ssip_edge());
    }

    #[test]
    fn install_returns_mtimer_index() {
        let mut bus = new_bus();
        let (aclint, _irq) = new_aclint();
        let mtimer_idx = aclint.install(&mut bus, ACLINT_BASE);
        bus.set_timer_source(mtimer_idx);
        // Drive enough ticks + a MtimeLo read to cross SYNC_INTERVAL.
        for _ in 0..1024 {
            bus.tick();
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
        let _ = bus.read(ACLINT_BASE + 0xBFF8, 4).unwrap();
        assert!(bus.mtime() > 0, "mtime must advance after ticks");
    }

    #[test]
    fn reset_clears_state_across_sub_devices() {
        let mut bus = new_bus();
        let (aclint, irq) = new_aclint();
        let mtimer_idx = aclint.install(&mut bus, ACLINT_BASE);
        bus.set_timer_source(mtimer_idx);

        bus.write(ACLINT_BASE + 0x0000, 4, 1).unwrap();
        bus.write(ACLINT_BASE + 0x4000, 4, 0).unwrap();
        bus.write(ACLINT_BASE + 0x4004, 4, 0).unwrap();
        bus.write(ACLINT_BASE + 0xC000, 4, 1).unwrap();
        bus.tick();
        assert_ne!(irq.load(), 0);

        bus.reset_devices();
        irq.reset();
        bus.tick();
        assert_eq!(irq.load() & MTIP, 0);
        assert_eq!(bus.read(ACLINT_BASE + 0x0000, 4).unwrap() as u32, 0);
        assert!(!irq.take_ssip_edge());
    }

    #[test]
    #[should_panic(expected = "overlaps")]
    fn install_panics_if_called_twice() {
        let mut bus = new_bus();
        let (a, _) = new_aclint();
        let _ = a.install(&mut bus, ACLINT_BASE);
        let (b, _) = new_aclint();
        let _ = b.install(&mut bus, ACLINT_BASE);
    }

    #[test]
    fn install_byte_compat_offsets() {
        // Spot-check each guest-visible global offset lands on the right region.
        let mut bus = new_bus();
        let (aclint, irq) = new_aclint();
        let mtimer_idx = aclint.install(&mut bus, ACLINT_BASE);
        bus.set_timer_source(mtimer_idx);

        bus.write(ACLINT_BASE + 0x0000, 4, 1).unwrap();
        assert_ne!(irq.load() & MSIP, 0);
        bus.write(ACLINT_BASE + 0x0000, 4, 0).unwrap();

        bus.write(ACLINT_BASE + 0x4000, 4, u32::MAX as Word)
            .unwrap();
        bus.write(ACLINT_BASE + 0x4004, 4, u32::MAX as Word)
            .unwrap();
        bus.tick();
        assert_eq!(irq.load() & MTIP, 0);

        bus.write(ACLINT_BASE + 0xC000, 4, 1).unwrap();
        assert!(irq.take_ssip_edge());
    }
}
