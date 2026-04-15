//! A extension: atomic LR/SC and AMO (.w/.d) handlers.
//!
//! Every handler takes `bus: &mut Bus` as the first parameter after `self`
//! (uniform dispatch-macro signature). LR/SC and AMO sequences hold the
//! exclusive `&mut Bus` borrow across translate + reservation-check +
//! conditional-store, which is precisely the peer-hart exclusion window
//! the mutex used to provide (invariant I-4 in `03_PLAN.md`).

// Word-to-u32 casts/masks are no-ops on isa32 but needed on isa64.
#![allow(clippy::identity_op, clippy::unnecessary_cast)]
// On isa32 the rv64_only! bodies collapse to `InvalidInst`, leaving bus-params unused.
#![allow(unused_variables)]

use super::{RVCore, rv64_only};
#[cfg(isa64)]
use crate::config::SWord;
use crate::{
    arch::riscv::cpu::mm::MemOp, config::Word, device::bus::Bus, error::XResult, isa::RVReg,
    utils::sext_word,
};

// --- Helpers ---

impl RVCore {
    fn amo_addr(&self, rs1: RVReg) -> memory_addr::VirtAddr {
        self.eff_addr(rs1, 0)
    }

    fn amo_w<F: FnOnce(u32, u32) -> u32>(
        &mut self,
        bus: &mut Bus,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
        op: F,
    ) -> XResult {
        let addr = self.amo_addr(rs1);
        let old = self.amo_load(bus, addr, 4)? & 0xFFFF_FFFF;
        self.amo_store(bus, addr, 4, op(old as u32, self.gpr[rs2] as u32) as Word)?;
        bus.clear_reservation(self.id);
        self.set_gpr(rd, sext_word(old, 32))
    }

    #[cfg(isa64)]
    fn amo_d<F: FnOnce(Word, Word) -> Word>(
        &mut self,
        bus: &mut Bus,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
        op: F,
    ) -> XResult {
        let addr = self.amo_addr(rs1);
        let old = self.amo_load(bus, addr, 8)?;
        self.amo_store(bus, addr, 8, op(old, self.gpr[rs2]))?;
        // See `amo_w` — own-hart reservation clear is conservative and matches
        // the spec rule that AMOs are not paired with `lr/sc`.
        bus.clear_reservation(self.id);
        self.set_gpr(rd, old)
    }
}

// --- LR/SC ---

impl RVCore {
    pub(super) fn lr_w(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        let addr = self.amo_addr(rs1);
        let paddr = self.translate(bus, addr, 4, MemOp::Load)?;
        let val = self.load(bus, addr, 4)? & 0xFFFF_FFFF;
        bus.reserve(self.id, paddr);
        self.set_gpr(rd, sext_word(val, 32))
    }

    pub(super) fn sc_w(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let addr = self.amo_addr(rs1);
        let paddr = self.translate(bus, addr, 4, MemOp::Store)?;
        let success = {
            let ok = bus.reservation(self.id) == Some(paddr);
            bus.clear_reservation(self.id);
            ok
        };
        if success {
            self.store(bus, addr, 4, self.gpr[rs2] & 0xFFFF_FFFF)?;
        }
        self.set_gpr(rd, !success as Word)
    }

    pub(super) fn lr_d(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        rv64_only!({
            let addr = self.amo_addr(rs1);
            let paddr = self.translate(bus, addr, 8, MemOp::Load)?;
            let val = self.load(bus, addr, 8)?;
            bus.reserve(self.id, paddr);
            self.set_gpr(rd, val)
        }; rd, rs1)
    }

    pub(super) fn sc_d(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_only!({
            let addr = self.amo_addr(rs1);
            let paddr = self.translate(bus, addr, 8, MemOp::Store)?;
            let success = {
                let ok = bus.reservation(self.id) == Some(paddr);
                bus.clear_reservation(self.id);
                ok
            };
            if success {
                self.store(bus, addr, 8, self.gpr[rs2])?;
            }
            self.set_gpr(rd, !success as Word)
        }; rd, rs1, rs2)
    }
}

// --- AMO .w (32-bit) ---

impl RVCore {
    pub(super) fn amoswap_w(
        &mut self,
        bus: &mut Bus,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
    ) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, |_, src| src)
    }
    pub(super) fn amoadd_w(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, u32::wrapping_add)
    }
    pub(super) fn amoxor_w(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, |old, src| old ^ src)
    }
    pub(super) fn amoand_w(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, |old, src| old & src)
    }
    pub(super) fn amoor_w(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, |old, src| old | src)
    }
    pub(super) fn amomin_w(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, |old, src| {
            (old as i32).min(src as i32) as u32
        })
    }
    pub(super) fn amomax_w(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, |old, src| {
            (old as i32).max(src as i32) as u32
        })
    }
    pub(super) fn amominu_w(
        &mut self,
        bus: &mut Bus,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
    ) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, u32::min)
    }
    pub(super) fn amomaxu_w(
        &mut self,
        bus: &mut Bus,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
    ) -> XResult {
        self.amo_w(bus, rd, rs1, rs2, u32::max)
    }
}

// --- AMO .d (64-bit) ---

/// RV64-only AMO .d: guard + dispatch to `amo_d`.
macro_rules! amo_d_op {
    ($self:ident, $bus:ident, $rd:ident, $rs1:ident, $rs2:ident, $op:expr) => {
        rv64_only!($self.amo_d($bus, $rd, $rs1, $rs2, $op); $rd, $rs1, $rs2)
    };
}

impl RVCore {
    pub(super) fn amoswap_d(
        &mut self,
        bus: &mut Bus,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
    ) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, |_, src| src)
    }
    pub(super) fn amoadd_d(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, Word::wrapping_add)
    }
    pub(super) fn amoxor_d(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, |old, src| old ^ src)
    }
    pub(super) fn amoand_d(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, |old, src| old & src)
    }
    pub(super) fn amoor_d(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, |old, src| old | src)
    }
    pub(super) fn amomin_d(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, |old: Word, src: Word| {
            (old as SWord).min(src as SWord) as Word
        })
    }
    pub(super) fn amomax_d(&mut self, bus: &mut Bus, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, |old: Word, src: Word| {
            (old as SWord).max(src as SWord) as Word
        })
    }
    pub(super) fn amominu_d(
        &mut self,
        bus: &mut Bus,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
    ) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, Word::min)
    }
    pub(super) fn amomaxu_d(
        &mut self,
        bus: &mut Bus,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
    ) -> XResult {
        amo_d_op!(self, bus, rd, rs1, rs2, Word::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        arch::riscv::cpu::trap::{Exception, TrapCause, test_helpers::assert_trap},
        config::{CONFIG_MBASE, CONFIG_MSIZE},
    };

    const fn addr(slot: usize) -> usize {
        CONFIG_MBASE + 0x2000 + slot * 0x100
    }

    fn setup_core(slot: usize, mem_val: Word, size: usize) -> (RVCore, Bus, usize) {
        let a = addr(slot);
        let mut core = RVCore::new();
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE, 1);
        core.gpr[RVReg::t0] = a as Word;
        bus.write(a, size, mem_val).unwrap();
        (core, bus, a)
    }

    fn read_mem(bus: &mut Bus, addr: usize, size: usize) -> Word {
        bus.read(addr, size).unwrap()
    }

    fn set_reservation(core: &RVCore, bus: &mut Bus, addr: Option<usize>) {
        match addr {
            Some(a) => bus.reserve(core.id, a),
            None => bus.clear_reservation(core.id),
        }
    }

    fn get_reservation(core: &RVCore, bus: &Bus) -> Option<usize> {
        bus.reservation(core.id)
    }

    // --- AMO .w ---

    #[test]
    fn amoadd_w_loads_old_and_stores_sum() {
        let (mut core, mut bus, a) = setup_core(0, 100, 4);
        core.gpr[RVReg::t1] = 42;
        core.amoadd_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t2], 100);
        assert_eq!(read_mem(&mut bus, a, 4), 142);
    }

    #[test]
    fn amoswap_w_replaces_value() {
        let (mut core, mut bus, a) = setup_core(1, 0xAAAA, 4);
        core.gpr[RVReg::t1] = 0xBBBB;
        core.amoswap_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0xAAAA);
        assert_eq!(read_mem(&mut bus, a, 4), 0xBBBB);
    }

    #[test]
    fn amoxor_w_xors_value() {
        let (mut core, mut bus, a) = setup_core(2, 0xFF00, 4);
        core.gpr[RVReg::t1] = 0x0FF0;
        core.amoxor_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0xFF00);
        assert_eq!(read_mem(&mut bus, a, 4), 0xF0F0);
    }

    #[test]
    fn amoand_w_and_amoor_w() {
        let (mut core, mut bus, a) = setup_core(3, 0xFF, 4);
        core.gpr[RVReg::t1] = 0x0F;
        core.amoand_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0xFF);
        assert_eq!(read_mem(&mut bus, a, 4), 0x0F);

        core.gpr[RVReg::t1] = 0xF0;
        core.amoor_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0x0F);
        assert_eq!(read_mem(&mut bus, a, 4), 0xFF);
    }

    #[test]
    fn amomin_w_signed_comparison() {
        let (mut core, mut bus, a) = setup_core(4, (-5_i32) as u32 as Word, 4);
        core.gpr[RVReg::t1] = 3;
        core.amomin_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(read_mem(&mut bus, a, 4) as u32, (-5_i32) as u32);
    }

    #[test]
    fn amomax_w_signed_comparison() {
        let (mut core, mut bus, a) = setup_core(5, (-5_i32) as u32 as Word, 4);
        core.gpr[RVReg::t1] = 3;
        core.amomax_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(read_mem(&mut bus, a, 4) as u32, 3);
    }

    #[test]
    fn amominu_w_unsigned_comparison() {
        let (mut core, mut bus, a) = setup_core(6, 0xFFFF_FFFF, 4);
        core.gpr[RVReg::t1] = 1;
        core.amominu_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(read_mem(&mut bus, a, 4), 1);
    }

    #[test]
    fn amomaxu_w_unsigned_comparison() {
        let (mut core, mut bus, a) = setup_core(7, 0xFFFF_FFFF, 4);
        core.gpr[RVReg::t1] = 1;
        core.amomaxu_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(read_mem(&mut bus, a, 4) as u32, 0xFFFF_FFFF);
    }

    #[test]
    #[cfg(isa64)]
    fn amo_w_sign_extends_on_rv64() {
        let (mut core, mut bus, _) = setup_core(8, 0x8000_0000, 4);
        core.gpr[RVReg::t1] = 0;
        core.amoadd_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0xFFFF_FFFF_8000_0000);
    }

    #[test]
    fn amo_clears_reservation() {
        let (mut core, mut bus, a) = setup_core(9, 0, 4);
        set_reservation(&core, &mut bus, Some(a));
        core.gpr[RVReg::t1] = 1;
        core.amoadd_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert!(get_reservation(&core, &bus).is_none());
    }

    #[test]
    fn amo_w_rd_zero_discards_but_operates() {
        let (mut core, mut bus, a) = setup_core(13, 10, 4);
        core.gpr[RVReg::t1] = 5;
        core.amoadd_w(&mut bus, RVReg::zero, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(core.gpr[RVReg::zero], 0);
        assert_eq!(read_mem(&mut bus, a, 4), 15);
    }

    // --- LR/SC ---

    #[test]
    fn lr_sc_w_success_path() {
        let (mut core, mut bus, a) = setup_core(10, 42, 4);
        core.lr_w(&mut bus, RVReg::t1, RVReg::t0, RVReg::zero)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t1], 42);
        assert_eq!(get_reservation(&core, &bus), Some(a));

        core.gpr[RVReg::t2] = 99;
        core.sc_w(&mut bus, RVReg::t3, RVReg::t0, RVReg::t2)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t3], 0); // success
        assert_eq!(read_mem(&mut bus, a, 4), 99);
        assert!(get_reservation(&core, &bus).is_none());
    }

    #[test]
    fn lr_sc_w_failure_path() {
        let (mut core, mut bus, a) = setup_core(11, 42, 4);
        core.lr_w(&mut bus, RVReg::t1, RVReg::t0, RVReg::zero)
            .unwrap();
        set_reservation(&core, &mut bus, None);

        core.gpr[RVReg::t2] = 99;
        core.sc_w(&mut bus, RVReg::t3, RVReg::t0, RVReg::t2)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t3], 1); // failure
        assert_eq!(read_mem(&mut bus, a, 4), 42); // unchanged
    }

    #[test]
    fn sc_w_without_lr_fails() {
        let (mut core, mut bus, a) = setup_core(12, 42, 4);
        core.gpr[RVReg::t1] = 99;
        core.sc_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t2], 1);
        assert_eq!(read_mem(&mut bus, a, 4), 42);
    }

    #[test]
    fn regular_store_invalidates_reservation() {
        let (mut core, mut bus, a) = setup_core(18, 42, 4);
        core.lr_w(&mut bus, RVReg::t1, RVReg::t0, RVReg::zero)
            .unwrap();
        assert!(get_reservation(&core, &bus).is_some());

        core.gpr[RVReg::t2] = 77;
        core.sw(&mut bus, RVReg::t0, RVReg::t2, 0).unwrap();

        core.gpr[RVReg::t2] = 99;
        core.sc_w(&mut bus, RVReg::t3, RVReg::t0, RVReg::t2)
            .unwrap();
        assert_eq!(core.gpr[RVReg::t3], 1); // SC must fail
        assert_eq!(read_mem(&mut bus, a, 4), 77);
    }

    #[test]
    fn lr_w_misaligned_returns_load_trap() {
        let mut core = RVCore::new();
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE, 1);
        let addr = CONFIG_MBASE + 0x2002;
        core.gpr[RVReg::t0] = addr as Word;

        assert_trap(
            core.lr_w(&mut bus, RVReg::t1, RVReg::t0, RVReg::zero),
            TrapCause::Exception(Exception::LoadMisaligned),
            addr as Word,
        );
    }

    #[test]
    fn amo_w_misaligned_returns_store_trap() {
        let mut core = RVCore::new();
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE, 1);
        let addr = CONFIG_MBASE + 0x2102;
        core.gpr[RVReg::t0] = addr as Word;
        core.gpr[RVReg::t1] = 1;

        assert_trap(
            core.amoadd_w(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1),
            TrapCause::Exception(Exception::StoreMisaligned),
            addr as Word,
        );
    }

    // --- RV64 .d ---

    #[cfg(isa64)]
    mod rv64_tests {
        use super::*;

        #[test]
        fn amoadd_d_loads_old_and_stores_sum() {
            let (mut core, mut bus, a) = setup_core(14, 0x1_0000_0000, 8);
            core.gpr[RVReg::t1] = 0x2_0000_0000;
            core.amoadd_d(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
                .unwrap();
            assert_eq!(core.gpr[RVReg::t2], 0x1_0000_0000);
            assert_eq!(read_mem(&mut bus, a, 8), 0x3_0000_0000);
        }

        #[test]
        fn lr_sc_d_success_path() {
            let (mut core, mut bus, a) = setup_core(15, 0xDEAD_BEEF_CAFE_BABE, 8);
            core.lr_d(&mut bus, RVReg::t1, RVReg::t0, RVReg::zero)
                .unwrap();
            assert_eq!(core.gpr[RVReg::t1], 0xDEAD_BEEF_CAFE_BABE);
            assert_eq!(get_reservation(&core, &bus), Some(a));

            core.gpr[RVReg::t2] = 0x1234_5678_9ABC_DEF0;
            core.sc_d(&mut bus, RVReg::t3, RVReg::t0, RVReg::t2)
                .unwrap();
            assert_eq!(core.gpr[RVReg::t3], 0);
            assert_eq!(read_mem(&mut bus, a, 8), 0x1234_5678_9ABC_DEF0);
        }

        #[test]
        fn amomin_d_signed_comparison() {
            let (mut core, mut bus, a) = setup_core(16, (-10_i64) as Word, 8);
            core.gpr[RVReg::t1] = 5;
            core.amomin_d(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
                .unwrap();
            assert_eq!(read_mem(&mut bus, a, 8) as SWord, -10);
        }

        #[test]
        fn amomaxu_d_unsigned_comparison() {
            let (mut core, mut bus, a) = setup_core(17, Word::MAX, 8);
            core.gpr[RVReg::t1] = 1;
            core.amomaxu_d(&mut bus, RVReg::t2, RVReg::t0, RVReg::t1)
                .unwrap();
            assert_eq!(read_mem(&mut bus, a, 8), Word::MAX);
        }
    }

    #[cfg(isa32)]
    mod rv32_tests {
        use super::*;
        use crate::error::XError;

        #[test]
        fn d_variants_rejected_on_rv32() {
            let mut core = RVCore::new();
            let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE, 1);
            for op in [
                RVCore::lr_d,
                RVCore::sc_d,
                RVCore::amoswap_d,
                RVCore::amoadd_d,
                RVCore::amoxor_d,
                RVCore::amoand_d,
                RVCore::amoor_d,
                RVCore::amomin_d,
                RVCore::amomax_d,
                RVCore::amominu_d,
                RVCore::amomaxu_d,
            ] {
                assert!(matches!(
                    op(&mut core, &mut bus, RVReg::t0, RVReg::t1, RVReg::t2),
                    Err(XError::InvalidInst)
                ));
            }
        }
    }
}
