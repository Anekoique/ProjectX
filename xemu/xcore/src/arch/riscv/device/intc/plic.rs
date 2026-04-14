//! Platform-Level Interrupt Controller (PLIC) with priority, enable, and
//! claim/complete semantics for M-mode and S-mode contexts.

use crate::{
    arch::riscv::cpu::trap::interrupt::{MEIP, SEIP},
    config::Word,
    device::{Device, IrqState},
    error::XResult,
};

const NUM_SRC: usize = 32;

const PRIORITY_END: usize = NUM_SRC * 4;
const PENDING_OFF: usize = 0x1000;
const ENABLE_BASE: usize = 0x2000;
const ENABLE_STRIDE: usize = 0x80;
const THRESHOLD_BASE: usize = 0x200000;
const CLAIM_BASE: usize = 0x200004;
const CTX_STRIDE: usize = 0x1000;

/// Platform-Level Interrupt Controller.
pub struct Plic {
    priority: Vec<u8>,
    pending: u32,
    num_ctx: usize,
    enable: Vec<u32>,
    threshold: Vec<u8>,
    claimed: Vec<u32>,
    irqs: Vec<IrqState>,
}

impl Plic {
    /// Create PLIC with per-hart IRQ states. `num_ctx = 2 * num_harts`
    /// (M-mode + S-mode per hart).
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self {
        debug_assert_eq!(irqs.len(), num_harts);
        let num_ctx = 2 * num_harts;
        Self {
            priority: vec![0; NUM_SRC],
            pending: 0,
            num_ctx,
            enable: vec![0; num_ctx],
            threshold: vec![0; num_ctx],
            claimed: vec![0; num_ctx],
            irqs,
        }
    }

    fn ctx_at(&self, offset: usize, base: usize, stride: usize) -> Option<usize> {
        let rel = offset.checked_sub(base)?;
        (rel / stride < self.num_ctx && rel.is_multiple_of(stride)).then_some(rel / stride)
    }

    /// Merge level-triggered device lines into pending.
    /// Sources claimed by any context are excluded from re-pending.
    fn update(&mut self, irq_lines: u32) {
        for src in 1..NUM_SRC {
            if self.is_claimed(src as u32) {
                continue;
            }
            let bit = 1u32 << src;
            if irq_lines & bit != 0 {
                self.pending |= bit;
            } else {
                self.pending &= !bit;
            }
        }
    }

    /// Claim: return highest-priority enabled pending source above threshold.
    /// Returns 0 if nothing qualifies or if this context has an outstanding
    /// claim.
    fn claim(&mut self, ctx: usize) -> u32 {
        if self.claimed[ctx] != 0 {
            return 0;
        }
        let result = (1..NUM_SRC)
            .filter(|&s| {
                self.pending & (1 << s) != 0
                    && self.enable[ctx] & (1 << s) != 0
                    && self.priority[s] > self.threshold[ctx]
            })
            .max_by_key(|&s| self.priority[s])
            .map(|s| {
                self.pending &= !(1 << s);
                self.claimed[ctx] = s as u32;
                debug!("plic: claim src={} for ctx={}", s, ctx);
                s as u32
            })
            .unwrap_or(0);
        self.evaluate();
        result
    }

    fn complete(&mut self, ctx: usize, src: u32) {
        if ctx < self.num_ctx && self.claimed[ctx] == src {
            debug!("plic: complete src={} for ctx={}", src, ctx);
            self.claimed[ctx] = 0;
        }
        self.evaluate();
    }

    fn is_claimed(&self, src: u32) -> bool {
        self.claimed.contains(&src)
    }

    fn evaluate(&mut self) {
        // Context layout: even = hart h M-mode (MEIP), odd = hart h S-mode (SEIP).
        for ctx in 0..self.num_ctx {
            let ip_bit = if ctx & 1 == 0 { MEIP } else { SEIP };
            let hart = ctx >> 1;
            let was_active = self.irqs[hart].load() & ip_bit != 0;
            let active = (1..NUM_SRC).any(|s| {
                self.pending & (1 << s) != 0
                    && self.enable[ctx] & (1 << s) != 0
                    && self.priority[s] > self.threshold[ctx]
            });
            if active {
                self.irqs[hart].set(ip_bit);
            } else {
                self.irqs[hart].clear(ip_bit);
            }
            if active != was_active {
                debug!(
                    "plic: ctx={} hart={} ip_bit={:#x} {} -> {}",
                    ctx, hart, ip_bit, was_active, active
                );
            }
        }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Plic {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match offset {
            o @ 0..PRIORITY_END if o.is_multiple_of(4) => self.priority[o / 4] as Word,
            PENDING_OFF => self.pending as Word,
            o => match self.ctx_at(o, ENABLE_BASE, ENABLE_STRIDE) {
                Some(c) => self.enable[c] as Word,
                None => match self.ctx_at(o, THRESHOLD_BASE, CTX_STRIDE) {
                    Some(c) => self.threshold[c] as Word,
                    None => match self.ctx_at(o, CLAIM_BASE, CTX_STRIDE) {
                        Some(c) => self.claim(c) as Word,
                        None => 0,
                    },
                },
            },
        })
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        match offset {
            o @ 0..PRIORITY_END if o.is_multiple_of(4) => self.priority[o / 4] = val as u8,
            o => {
                if let Some(c) = self.ctx_at(o, ENABLE_BASE, ENABLE_STRIDE) {
                    self.enable[c] = val as u32;
                } else if let Some(c) = self.ctx_at(o, THRESHOLD_BASE, CTX_STRIDE) {
                    self.threshold[c] = val as u8;
                    self.evaluate();
                } else if let Some(c) = self.ctx_at(o, CLAIM_BASE, CTX_STRIDE) {
                    self.complete(c, val as u32);
                }
            }
        }
        Ok(())
    }

    fn notify(&mut self, irq_lines: u32) {
        self.update(irq_lines);
        self.evaluate();
    }

    fn reset(&mut self) {
        self.priority.fill(0);
        self.pending = 0;
        self.enable.fill(0);
        self.threshold.fill(0);
        self.claimed.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Plic, IrqState) {
        let irq = IrqState::new();
        (Plic::new(1, vec![irq.clone()]), irq)
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
        p.notify(0x06);
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
        p.notify(0x06);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 2); // first claim
        p.notify(0x06); // re-pend source 1
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 0); // outstanding claim blocks
    }

    #[test]
    fn complete_releases_claimed() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.notify(0x02);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 1);
        p.write(0x200004, 4, 1).unwrap();
        assert_eq!(p.claimed[0], 0);
    }

    #[test]
    fn threshold_filters() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 3).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.write(0x200000, 4, 5).unwrap();
        p.notify(0x02);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 0);
    }

    #[test]
    fn claimed_source_not_repended() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.notify(0x02);
        p.read(0x200004, 4).unwrap();
        p.notify(0x02);
        assert_eq!(p.pending & 0x02, 0);
    }

    #[test]
    fn source_repended_after_complete() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.notify(0x02);
        p.read(0x200004, 4).unwrap();
        p.write(0x200004, 4, 1).unwrap();
        p.notify(0x02);
        assert_ne!(p.pending & 0x02, 0);
    }

    #[test]
    fn complete_wrong_source_no_change() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.notify(0x02);
        p.read(0x200004, 4).unwrap();
        p.write(0x200004, 4, 99).unwrap();
        assert_eq!(p.claimed[0], 1);
    }

    #[test]
    fn source_zero_excluded() {
        let (mut p, _) = setup();
        p.write(0x00, 4, 10).unwrap();
        p.write(0x2000, 4, 0x01).unwrap();
        p.notify(0x01);
        assert_eq!(p.read(0x200004, 4).unwrap() as u32, 0);
    }

    #[test]
    fn meip_seip_set_and_clear() {
        let (mut p, irq) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.write(0x2080, 4, 0x02).unwrap();
        p.notify(0x02);
        assert_ne!(irq.load() & MEIP, 0);
        assert_ne!(irq.load() & SEIP, 0);
        p.read(0x200004, 4).unwrap();
        p.write(0x200004, 4, 1).unwrap();
        p.read(0x201004, 4).unwrap();
        p.write(0x201004, 4, 1).unwrap();
        p.notify(0x00);
        assert_eq!(irq.load() & MEIP, 0);
        assert_eq!(irq.load() & SEIP, 0);
    }

    #[test]
    fn claim_clears_meip_when_last_source() {
        let (mut p, irq) = setup();
        p.write(0x04, 4, 1).unwrap();
        p.write(0x2000, 4, 0x02).unwrap();
        p.notify(0x02);
        assert_ne!(irq.load() & MEIP, 0);
        p.read(0x200004, 4).unwrap();
        assert_eq!(irq.load() & MEIP, 0);
    }

    #[test]
    fn reset_clears_state() {
        let (mut p, _) = setup();
        p.write(0x04, 4, 5).unwrap();
        p.write(0x2000, 4, 0xFF).unwrap();
        p.notify(0x02);
        p.reset();
        assert_eq!(p.pending, 0);
        assert_eq!(p.priority[1], 0);
        assert_eq!(p.enable[0], 0);
    }

    #[test]
    fn plic_new_num_harts_two_ctx2_routes_to_irq1() {
        // At num_harts=2, context 2 (M-mode for hart 1) must target irqs[1] MEIP.
        let irq0 = IrqState::new();
        let irq1 = IrqState::new();
        let mut p = Plic::new(2, vec![irq0.clone(), irq1.clone()]);

        // Source 1: priority=1, enable on context 2 (= hart 1 M-mode)
        p.write(0x04, 4, 1).unwrap(); // priority[1] = 1
        p.write(0x2000 + 2 * 0x80, 4, 0x02).unwrap(); // enable[ctx=2], bit 1

        // Raise source 1
        p.notify(0x02);

        // hart 0 should NOT see MEIP; hart 1 should.
        assert_eq!(irq0.load() & MEIP, 0, "hart 0 MEIP must stay clear");
        assert_ne!(irq1.load() & MEIP, 0, "hart 1 MEIP must be asserted");
    }
}
