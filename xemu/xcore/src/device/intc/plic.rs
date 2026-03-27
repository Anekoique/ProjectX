use crate::{
    config::Word,
    device::{Device, IrqState, MEIP, SEIP},
    error::XResult,
};

const NUM_SRC: usize = 32;
const NUM_CTX: usize = 2; // 0 = M-mode, 1 = S-mode

const PRIORITY_END: usize = NUM_SRC * 4;
const PENDING_OFF: usize = 0x001000;
const ENABLE_BASE: usize = 0x002000;
const ENABLE_STRIDE: usize = 0x80;
const ENABLE_END: usize = ENABLE_BASE + NUM_CTX * ENABLE_STRIDE;
const THRESHOLD_BASE: usize = 0x200000;
const CLAIM_BASE: usize = 0x200004;
const CTX_STRIDE: usize = 0x1000;

const CTX_IP: [u64; NUM_CTX] = [MEIP, SEIP];

pub struct Plic {
    priority: Vec<u8>,
    pending: u32,
    enable: Vec<u32>,
    threshold: Vec<u8>,
    claimed: Vec<u32>,
    irq: IrqState,
}

impl Plic {
    pub fn new(irq: IrqState) -> Self {
        Self {
            priority: vec![0; NUM_SRC],
            pending: 0,
            enable: vec![0; NUM_CTX],
            threshold: vec![0; NUM_CTX],
            claimed: vec![0; NUM_CTX],
            irq,
        }
    }

    fn ctx_of(offset: usize, base: usize, stride: usize) -> Option<usize> {
        let c = offset.checked_sub(base)? / stride;
        (c < NUM_CTX).then_some(c)
    }

    fn is_threshold(o: usize) -> bool {
        o >= THRESHOLD_BASE
            && (o - THRESHOLD_BASE) / CTX_STRIDE < NUM_CTX
            && (o - THRESHOLD_BASE).is_multiple_of(CTX_STRIDE)
    }

    fn is_claim(o: usize) -> bool {
        o >= CLAIM_BASE
            && (o - CLAIM_BASE) / CTX_STRIDE < NUM_CTX
            && (o - CLAIM_BASE).is_multiple_of(CTX_STRIDE)
    }

    /// Merge level-triggered device lines into pending.
    /// Sources claimed by any context are excluded from re-pending.
    fn update(&mut self, irq_lines: u32) {
        for src in 1..NUM_SRC {
            let bit = 1u32 << src;
            if self.is_claimed(src as u32) {
                continue;
            }
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
                s as u32
            })
            .unwrap_or(0);
        self.evaluate();
        result
    }

    fn complete(&mut self, ctx: usize, src: u32) {
        if ctx < NUM_CTX && self.claimed[ctx] == src {
            self.claimed[ctx] = 0;
        }
        self.evaluate();
    }

    fn is_claimed(&self, src: u32) -> bool {
        self.claimed.contains(&src)
    }

    fn evaluate(&mut self) {
        for (ctx, &ip) in CTX_IP.iter().enumerate() {
            let active = (1..NUM_SRC).any(|s| {
                self.pending & (1 << s) != 0
                    && self.enable[ctx] & (1 << s) != 0
                    && self.priority[s] > self.threshold[ctx]
            });
            if active {
                self.irq.set(ip);
            } else {
                self.irq.clear(ip);
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
            o if (ENABLE_BASE..ENABLE_END).contains(&o)
                && (o - ENABLE_BASE).is_multiple_of(ENABLE_STRIDE) =>
            {
                Self::ctx_of(o, ENABLE_BASE, ENABLE_STRIDE).map_or(0, |c| self.enable[c] as Word)
            }
            o if Self::is_threshold(o) => {
                Self::ctx_of(o, THRESHOLD_BASE, CTX_STRIDE).map_or(0, |c| self.threshold[c] as Word)
            }
            o if Self::is_claim(o) => {
                Self::ctx_of(o, CLAIM_BASE, CTX_STRIDE).map_or(0, |c| self.claim(c) as Word)
            }
            _ => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        match offset {
            o @ 0..PRIORITY_END if o.is_multiple_of(4) => self.priority[o / 4] = val as u8,
            o if (ENABLE_BASE..ENABLE_END).contains(&o)
                && (o - ENABLE_BASE).is_multiple_of(ENABLE_STRIDE) =>
            {
                if let Some(c) = Self::ctx_of(o, ENABLE_BASE, ENABLE_STRIDE) {
                    self.enable[c] = val as u32;
                }
            }
            o if Self::is_threshold(o) => {
                if let Some(c) = Self::ctx_of(o, THRESHOLD_BASE, CTX_STRIDE) {
                    self.threshold[c] = val as u8;
                    self.evaluate();
                }
            }
            o if Self::is_claim(o) => {
                if let Some(c) = Self::ctx_of(o, CLAIM_BASE, CTX_STRIDE) {
                    self.complete(c, val as u32);
                }
            }
            _ => {}
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
        (Plic::new(irq.clone()), irq)
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
}
