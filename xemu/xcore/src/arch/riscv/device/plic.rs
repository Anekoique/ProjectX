//! Platform-Level Interrupt Controller (PLIC).
//!
//! Behind the MMIO facade:
//! - [`gateway`] — per-source level FSM (SiFive pre-claim-clear variant).
//! - [`core`] — priority/pending/enable/threshold/claim arbitration, MEIP/SEIP
//!   drive.
//! - `Plic` (this module) — MMIO decode, orchestration, and signal-plane drain.
//!   Devices signal via [`IrqLine`] (see `device::irq`); `Plic::tick` drains
//!   the plane and feeds gateway FSMs (directIrq 02_PLAN).

mod core;
mod gateway;

use std::sync::Arc;

use self::{
    core::{Core, NUM_SRC},
    gateway::{Gateway, GatewayDecision},
};
use crate::{
    config::Word,
    device::{Device, IrqLine, IrqState, irq::PlicSignals},
    error::XResult,
};

// Width of the level bitmap in `PlicSignals` (directIrq I-D12).
const _: () = assert!(
    NUM_SRC <= 32,
    "PlicSignals bitmap is u32; NUM_SRC must be <= 32"
);

const PRIORITY_END: usize = NUM_SRC * 4;
const PENDING_OFF: usize = 0x1000;
const ENABLE_BASE: usize = 0x2000;
const ENABLE_STRIDE: usize = 0x80;
const THRESHOLD_BASE: usize = 0x200000;
const CLAIM_BASE: usize = 0x200004;
const CTX_STRIDE: usize = 0x1000;

/// Platform-Level Interrupt Controller.
pub struct Plic {
    gateways: [Gateway; NUM_SRC],
    core: Core,
    /// Shared atomic signal plane — see `device::irq`. The `Arc` identity is
    /// stable across [`Plic::reset`] so device-held [`IrqLine`] handles
    /// remain valid for the lifetime of the PLIC (directIrq I-D8 / I-D8a).
    signals: Arc<PlicSignals>,
}

impl Plic {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self {
        Self {
            gateways: [(); NUM_SRC].map(|()| Gateway::new()),
            core: Core::new(num_harts, irqs),
            signals: Arc::new(PlicSignals::new()),
        }
    }

    /// Mint an [`IrqLine`] handle bound to `src`. Devices call this once at
    /// machine construction and hand the returned handle to the device.
    pub fn with_irq_line(&self, src: u32) -> IrqLine {
        IrqLine::new(self.signals.clone(), src)
    }

    /// Decode `offset` as a context-indexed register in `[base, base + num_ctx
    /// * stride)`.
    fn ctx_at(&self, offset: usize, base: usize, stride: usize) -> Option<usize> {
        let rel = offset.checked_sub(base)?;
        (rel / stride < self.core.num_ctx() && rel.is_multiple_of(stride)).then_some(rel / stride)
    }

    /// Guest read of the claim register for `ctx`: delegate to `core`, then
    /// notify the gateway it went in-flight.
    fn read_claim(&mut self, ctx: usize) -> u32 {
        let src = self.core.claim(ctx);
        if src != 0 {
            self.gateways[src as usize].on_claim();
        }
        src
    }

    /// Guest write to the complete register for `ctx`: if the claim matches,
    /// let the gateway decide whether to re-pend within the same MMIO frame.
    fn write_complete(&mut self, ctx: usize, src: u32) {
        if !self.core.complete(ctx, src) {
            return;
        }
        let s = src as usize;
        debug_assert!((1..NUM_SRC).contains(&s), "complete src out of range: {s}");
        if matches!(self.gateways[s].on_complete(), GatewayDecision::Pend) {
            self.core.set_pending(s);
            self.core.evaluate();
        }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Plic {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match offset {
            o @ 0..PRIORITY_END if o.is_multiple_of(4) => self.core.priority(o / 4) as Word,
            PENDING_OFF => self.core.pending_bits() as Word,
            o => match self.ctx_at(o, ENABLE_BASE, ENABLE_STRIDE) {
                Some(c) => self.core.enable(c) as Word,
                None => match self.ctx_at(o, THRESHOLD_BASE, CTX_STRIDE) {
                    Some(c) => self.core.threshold(c) as Word,
                    None => match self.ctx_at(o, CLAIM_BASE, CTX_STRIDE) {
                        Some(c) => self.read_claim(c) as Word,
                        None => 0,
                    },
                },
            },
        })
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        // PENDING_OFF (0x1000) is read-only per PLIC spec; guest writes fall
        // through to the `ctx_at` chain below and are silently ignored. Every
        // mutation re-runs `core.evaluate()` so MEIP/SEIP reflect the new
        // state in the same MMIO frame (PLIC v1.0.0 §1.3).
        match offset {
            o @ 0..PRIORITY_END if o.is_multiple_of(4) => {
                self.core.set_priority(o / 4, val as u8);
                self.core.evaluate();
            }
            o => {
                if let Some(c) = self.ctx_at(o, ENABLE_BASE, ENABLE_STRIDE) {
                    self.core.set_enable(c, val as u32);
                    self.core.evaluate();
                } else if let Some(c) = self.ctx_at(o, THRESHOLD_BASE, CTX_STRIDE) {
                    self.core.set_threshold(c, val as u8);
                    self.core.evaluate();
                } else if let Some(c) = self.ctx_at(o, CLAIM_BASE, CTX_STRIDE) {
                    self.write_complete(c, val as u32);
                }
            }
        }
        Ok(())
    }

    /// Drain the signal plane and feed gateway FSMs. `Bus::tick` calls this
    /// after every other device's `tick`, so raises that happen inside this
    /// same slow-tick are observed in one pass (directIrq I-D16). Fast path:
    /// one `Acquire` swap and zero per-source work when no raise is pending.
    fn tick(&mut self) {
        let Some(level) = self.signals.drain() else {
            return;
        };
        for s in 1..NUM_SRC {
            match self.gateways[s].sample(level & (1u32 << s) != 0) {
                GatewayDecision::Pend => self.core.set_pending(s),
                GatewayDecision::Clear => self.core.clear_pending(s),
                GatewayDecision::NoChange => {}
            }
        }
        self.core.evaluate();
    }

    fn reset(&mut self) {
        self.core.reset_runtime();
        for g in &mut self.gateways {
            g.reset_runtime();
        }
        self.signals.reset();
        // Drive IRQ lines low: reset_runtime zeroed pending/enable, but the
        // IrqState bits are only recomputed inside evaluate().
        self.core.evaluate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::riscv::cpu::trap::interrupt::{MEIP, SEIP};

    fn setup() -> (Plic, IrqState) {
        let irq = IrqState::new();
        (Plic::new(1, vec![irq.clone()]), irq)
    }

    /// Drive all source lines to match `bitmap`, then tick so the PLIC
    /// observes the new state.
    fn drive(p: &mut Plic, bitmap: u32) {
        for s in 1..NUM_SRC as u32 {
            let line = p.with_irq_line(s);
            if bitmap & (1 << s) != 0 {
                line.raise();
            } else {
                line.lower();
            }
        }
        p.tick();
    }

    #[test]
    fn priority_read_write() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 7).unwrap();
        assert_eq!(p.read(0x04, 4).unwrap() as u8, 7);
    }

    #[test]
    fn enable_per_context() {
        let (mut p, _) = setup();
        p.write(0x2000, 4, 0xFF).unwrap();
        p.write(0x2080, 4, 0x0F).unwrap();
        assert_eq!(p.read(0x2000, 4).unwrap() as u32, 0xFF);
        assert_eq!(p.read(0x2080, 4).unwrap() as u32, 0x0F);
    }

    #[test]
    fn claim_highest_priority() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 3).unwrap();
        p.write(0x08, 4, 5).unwrap();
        p.write(0x2000, 4, 0x06).unwrap();
        drive(&mut p, 0x06);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 2);
    }

    #[test]
    fn claim_empty_returns_zero() {
        let (mut p, _) = setup();
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 0);
    }

    #[test]
    fn double_claim_returns_zero() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x08, 4, 2).unwrap();
        p.write(0x2000, 4, 0x06).unwrap();
        drive(&mut p, 0x06);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 2); // first claim
        drive(&mut p, 0x06); // re-pend source 1
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 0); // outstanding claim blocks
    }

    #[test]
    fn complete_releases_claimed() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        drive(&mut p, 0x02);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 1);
        p.write(0x200004, 4, 1).unwrap();
        // Claim slot freed: a fresh pend can claim again.
        drive(&mut p, 0x02);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 1);
    }

    #[test]
    fn threshold_filters() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 3).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.write(0x200000, 4, 5).unwrap();
        drive(&mut p, 0x02);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 0);
    }

    #[test]
    fn claimed_source_not_repended() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        drive(&mut p, 0x02);
        p.read(0x200004, 4).unwrap();
        // Line still high, but source is in-flight: pending register reads 0.
        drive(&mut p, 0x02);
        assert_eq!(p.read(PENDING_OFF, 4).unwrap() as u32 & 0x02, 0);
    }

    #[test]
    fn source_repended_after_complete() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        drive(&mut p, 0x02);
        p.read(0x200004, 4).unwrap();
        p.write(0x200004, 4, 1).unwrap();
        drive(&mut p, 0x02);
        assert_ne!(p.read(PENDING_OFF, 4).unwrap() as u32 & 0x02, 0);
    }

    #[test]
    fn complete_unenabled_source_silently_ignored() {
        // PLIC v1.0.0 §9: a completion whose id is not enabled for the target
        // is silently ignored.
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap(); // only source 1 is enabled
        drive(&mut p, 0x02);
        p.read(0x200004, 4).unwrap();
        p.write(0x200004, 4, 99).unwrap(); // src=99: not enabled
        // Claim still outstanding: ctx 0 cannot claim again until a valid
        // complete arrives.
        drive(&mut p, 0x02);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 0);
    }

    #[test]
    fn source_zero_excluded() {
        let (mut p, _) = setup();
        p.write(0x00, 4, 10).unwrap();
        p.write(0x2000, 4, 0x01).unwrap();
        drive(&mut p, 0x01);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 0);
    }

    #[test]
    fn meip_seip_set_and_clear() {
        let (mut p, irq) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.write(0x2080, 4, 0x02).unwrap();
        drive(&mut p, 0x02);
        assert_ne!(irq.load() & MEIP, 0);
        assert_ne!(irq.load() & SEIP, 0);
        p.read(0x200004, 4).unwrap();
        p.write(0x200004, 4, 1).unwrap();
        p.read(0x201004, 4).unwrap();
        p.write(0x201004, 4, 1).unwrap();
        drive(&mut p, 0x00);
        assert_eq!(irq.load() & MEIP, 0);
        assert_eq!(irq.load() & SEIP, 0);
    }

    #[test]
    fn claim_clears_meip_when_last_source() {
        let (mut p, irq) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        drive(&mut p, 0x02);
        assert_ne!(irq.load() & MEIP, 0);
        p.read(0x200004, 4).unwrap();
        assert_eq!(irq.load() & MEIP, 0);
    }

    #[test]
    fn reset_clears_state_and_lowers_irq() {
        let (mut p, irq) = setup();
        p.write(0x04, 4, 5).unwrap();
        p.write(0x2000, 4, 0xFF).unwrap();
        drive(&mut p, 0x02);
        assert_ne!(irq.load() & MEIP, 0);
        p.reset();
        assert_eq!(p.read(0x04, 4).unwrap() as u32, 0);
        assert_eq!(p.read(0x2000, 4).unwrap() as u32, 0);
        assert_eq!(p.read(PENDING_OFF, 4).unwrap() as u32, 0);
        assert_eq!(irq.load() & MEIP, 0);
    }

    #[test]
    fn plic_new_num_harts_two_ctx2_routes_to_irq1() {
        // At num_harts=2, context 2 (M-mode for hart 1) must target irqs[1] MEIP.
        let irq0 = IrqState::new();
        let irq1 = IrqState::new();
        let mut p = Plic::new(2, vec![irq0.clone(), irq1.clone()]);

        p.write(0x04, 4, 1).unwrap(); // priority[1] = 1
        p.write(0x2000 + 2 * 0x80, 4, 0x02).unwrap(); // enable[ctx=2], bit 1
        drive(&mut p, 0x02);

        assert_eq!(irq0.load() & MEIP, 0, "hart 0 MEIP must stay clear");
        assert_ne!(irq1.load() & MEIP, 0, "hart 1 MEIP must be asserted");
    }
}
