//! PLIC arbitration core: per-source priority/pending, per-context
//! enable/threshold/claim state, and MEIP/SEIP IRQ line drive.
//!
//! The core is gateway-agnostic (02_PLAN Design (a)). `Plic::read`/`write`
//! drive gateway callbacks around `claim`/`complete`; nothing in this module
//! references `Gateway`.

use crate::{
    arch::riscv::cpu::trap::interrupt::{MEIP, SEIP},
    device::IrqState,
};

/// Number of supported interrupt sources (source 0 is reserved).
pub(super) const NUM_SRC: usize = 32;

/// PLIC arbitration state.
pub(super) struct Core {
    priority: [u8; NUM_SRC],
    pending: u32,
    num_ctx: usize,
    enable: Vec<u32>,
    threshold: Vec<u8>,
    claimed: Vec<u32>,
    irqs: Vec<IrqState>,
}

impl Core {
    pub(super) fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self {
        debug_assert_eq!(irqs.len(), num_harts);
        let num_ctx = 2 * num_harts;
        Self {
            priority: [0; NUM_SRC],
            pending: 0,
            num_ctx,
            enable: vec![0; num_ctx],
            threshold: vec![0; num_ctx],
            claimed: vec![0; num_ctx],
            irqs,
        }
    }

    pub(super) fn num_ctx(&self) -> usize {
        self.num_ctx
    }

    // Source-indexed accessors.

    pub(super) fn set_priority(&mut self, src: usize, prio: u8) {
        self.priority[src] = prio;
    }

    pub(super) fn priority(&self, src: usize) -> u8 {
        self.priority[src]
    }

    pub(super) fn set_pending(&mut self, src: usize) {
        self.pending |= 1u32 << src;
    }

    pub(super) fn clear_pending(&mut self, src: usize) {
        self.pending &= !(1u32 << src);
    }

    pub(super) fn pending_bits(&self) -> u32 {
        self.pending
    }

    // Context-indexed accessors.

    pub(super) fn set_enable(&mut self, ctx: usize, val: u32) {
        self.enable[ctx] = val;
    }

    pub(super) fn enable(&self, ctx: usize) -> u32 {
        self.enable[ctx]
    }

    pub(super) fn set_threshold(&mut self, ctx: usize, thr: u8) {
        self.threshold[ctx] = thr;
    }

    pub(super) fn threshold(&self, ctx: usize) -> u8 {
        self.threshold[ctx]
    }

    // Arbitration.

    /// Claim the highest-priority enabled pending source above threshold for
    /// `ctx`. Returns 0 if nothing qualifies or a claim is already outstanding.
    pub(super) fn claim(&mut self, ctx: usize) -> u32 {
        if self.claimed[ctx] != 0 {
            return 0;
        }
        let result = (1..NUM_SRC)
            .filter(|&s| self.selectable(ctx, s))
            .max_by_key(|&s| self.priority[s])
            .map(|s| {
                self.pending &= !(1u32 << s);
                self.claimed[ctx] = s as u32;
                debug!("plic: claim src={s} for ctx={ctx}");
                s as u32
            })
            .unwrap_or(0);
        self.evaluate();
        result
    }

    /// Acknowledge a complete for `src` on `ctx`. Returns true iff it matched
    /// the outstanding claim.
    pub(super) fn complete(&mut self, ctx: usize, src: u32) -> bool {
        if ctx >= self.num_ctx || self.claimed[ctx] != src {
            return false;
        }
        debug!("plic: complete src={src} for ctx={ctx}");
        self.claimed[ctx] = 0;
        self.evaluate();
        true
    }

    /// Recompute per-context IRQ line assertion. Even ctx = hart M-mode
    /// (MEIP), odd ctx = hart S-mode (SEIP).
    pub(super) fn evaluate(&mut self) {
        for ctx in 0..self.num_ctx {
            let ip_bit = if ctx & 1 == 0 { MEIP } else { SEIP };
            let hart = ctx >> 1;
            let was = self.irqs[hart].load() & ip_bit != 0;
            let now = (1..NUM_SRC).any(|s| self.selectable(ctx, s));
            if now {
                self.irqs[hart].set(ip_bit);
            } else {
                self.irqs[hart].clear(ip_bit);
            }
            if now != was {
                debug!("plic: ctx={ctx} hart={hart} ip_bit={ip_bit:#x} {was} -> {now}");
            }
        }
    }

    /// True iff the source is eligible to claim or drive its context's IRQ.
    fn selectable(&self, ctx: usize, src: usize) -> bool {
        self.pending & (1u32 << src) != 0
            && self.enable[ctx] & (1u32 << src) != 0
            && self.priority[src] > self.threshold[ctx]
    }

    /// Clear runtime state (priority, pending, enable, threshold, claimed).
    /// The caller drives `evaluate` afterwards to lower IRQ lines.
    pub(super) fn reset_runtime(&mut self) {
        self.priority.fill(0);
        self.pending = 0;
        self.enable.fill(0);
        self.threshold.fill(0);
        self.claimed.fill(0);
    }
}
