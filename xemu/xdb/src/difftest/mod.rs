//! Difftest framework — per-instruction state comparison against a reference
//! emulator.

pub mod gdb;
pub mod qemu;
pub mod spike;

use xcore::CoreContext;

// ── Backend trait ──

/// Reference emulator backend for per-instruction comparison.
pub trait DiffBackend {
    fn step(&mut self) -> Result<(), String>;
    fn read_context(&mut self) -> Result<CoreContext, String>;
    fn sync_state(&mut self, ctx: &CoreContext) -> Result<(), String>;
    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String>;
    fn name(&self) -> &str;
}

// ── Mismatch ──

/// A single register mismatch between DUT and REF.
pub struct DiffMismatch {
    pub inst_count: u64,
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}

// ── Harness ──

/// Orchestrates step-compare-sync against a reference backend.
pub struct DiffHarness {
    backend: Box<dyn DiffBackend>,
    inst_count: u64,
}

impl DiffHarness {
    /// Wrap a backend into a harness.
    pub fn new(backend: Box<dyn DiffBackend>) -> Self {
        Self {
            backend,
            inst_count: 0,
        }
    }

    /// Called after each DUT step. Steps REF, compares or syncs.
    pub fn check_step(
        &mut self,
        dut_ctx: &CoreContext,
        mmio: bool,
        halted: bool,
    ) -> Result<Option<DiffMismatch>, String> {
        self.inst_count += 1;
        self.backend.step()?;
        if mmio || halted {
            if mmio {
                debug!("difftest: MMIO skip at inst {}", self.inst_count);
            }
            return self.backend.sync_state(dut_ctx).map(|_| None);
        }
        self.backend
            .read_context()
            .map(|ref_ctx| diff_contexts(dut_ctx, &ref_ctx, self.inst_count))
    }

    /// Print a formatted mismatch report to stderr.
    pub fn report_mismatch(m: &DiffMismatch) {
        eprintln!(
            "DIFFTEST MISMATCH at instruction {}:\n  register: {}\n  DUT: {:#018x}\n  REF: \
             {:#018x}",
            m.inst_count, m.reg_name, m.dut_val, m.ref_val
        );
    }

    /// Number of instructions checked so far.
    pub fn inst_count(&self) -> u64 {
        self.inst_count
    }

    /// Name of the attached backend.
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }
}

// ── Comparison (free function — CoreContext defined in xcore) ──

/// Compare DUT and REF contexts, returning the first mismatch.
pub fn diff_contexts(
    dut: &CoreContext,
    refr: &CoreContext,
    inst_count: u64,
) -> Option<DiffMismatch> {
    let mismatch = |reg_name, dut_val, ref_val| {
        Some(DiffMismatch {
            inst_count,
            reg_name,
            dut_val,
            ref_val,
        })
    };

    if dut.pc != refr.pc {
        return mismatch("pc", dut.pc, refr.pc);
    }

    // GPR — skip x0, compare by position
    assert_eq!(dut.gprs.len(), refr.gprs.len(), "GPR count mismatch");
    for ((dname, dval), (_, rval)) in dut.gprs.iter().zip(&refr.gprs).skip(1) {
        if dval != rval {
            return mismatch(dname, *dval, *rval);
        }
    }

    // Privilege — skip when REF reports 0 (QEMU pseudo-register unreliable)
    if dut.privilege != refr.privilege && refr.privilege != 0 {
        return mismatch("privilege", dut.privilege, refr.privilege);
    }

    // CSRs — match by addr, symmetric check
    for &(addr, name, mask, raw) in &dut.csrs {
        let (d, r) = match refr.csrs.iter().find(|c| c.0 == addr) {
            Some(&(_, _, _, ref_raw)) => (raw & mask, ref_raw & mask),
            None => return mismatch(name, raw & mask, 0),
        };
        if d != r {
            return mismatch(name, d, r);
        }
    }
    for &(addr, name, mask, raw) in &refr.csrs {
        if !dut.csrs.iter().any(|c| c.0 == addr) {
            return mismatch(name, 0, raw & mask);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(pc: u64, gprs: &[u64], priv_mode: u64, csrs: &[(u16, u64)]) -> CoreContext {
        CoreContext {
            pc,
            gprs: gprs
                .iter()
                .enumerate()
                .map(|(i, &v)| (GPR_NAME[i], v))
                .collect(),
            privilege: priv_mode,
            csrs: csrs
                .iter()
                .map(|&(a, v)| (a, "test", u64::MAX, v))
                .collect(),
            word_size: 8,
            isa: "rv64imac",
        }
    }

    const GPR_NAME: [&str; 32] = [
        "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
        "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
        "t5", "t6",
    ];

    #[test]
    fn diff_identical() {
        let ctx = make_ctx(0x80000000, &[0; 32], 3, &[(0x300, 0)]);
        assert!(diff_contexts(&ctx, &ctx, 1).is_none());
    }

    #[test]
    fn diff_pc_mismatch() {
        let a = make_ctx(0x1000, &[0; 32], 3, &[]);
        let b = make_ctx(0x2000, &[0; 32], 3, &[]);
        let m = diff_contexts(&a, &b, 1).unwrap();
        assert_eq!(m.reg_name, "pc");
    }

    #[test]
    fn diff_gpr_mismatch() {
        let mut gprs_a = [0u64; 32];
        let mut gprs_b = [0u64; 32];
        gprs_a[5] = 42;
        gprs_b[5] = 99;
        let a = make_ctx(0x1000, &gprs_a, 3, &[]);
        let b = make_ctx(0x1000, &gprs_b, 3, &[]);
        let m = diff_contexts(&a, &b, 1).unwrap();
        assert_eq!(m.reg_name, "t0");
        assert_eq!(m.dut_val, 42);
        assert_eq!(m.ref_val, 99);
    }

    #[test]
    fn diff_privilege_mismatch() {
        let a = make_ctx(0x1000, &[0; 32], 3, &[]);
        let b = make_ctx(0x1000, &[0; 32], 1, &[]);
        let m = diff_contexts(&a, &b, 1).unwrap();
        assert_eq!(m.reg_name, "privilege");
    }

    #[test]
    fn diff_csr_mismatch() {
        let a = make_ctx(0x1000, &[0; 32], 3, &[(0x300, 0xAA)]);
        let b = make_ctx(0x1000, &[0; 32], 3, &[(0x300, 0xBB)]);
        let m = diff_contexts(&a, &b, 1).unwrap();
        assert_eq!(m.reg_name, "test");
    }

    #[test]
    fn diff_csr_masked() {
        // mip with mask !0x82 — bits 1 and 7 ignored
        let a = CoreContext {
            pc: 0x1000,
            gprs: (0..32).map(|i| (GPR_NAME[i], 0u64)).collect(),
            privilege: 3,
            csrs: vec![(0x344, "mip", !0x82_u64, 0xFF)],
            word_size: 8,
            isa: "rv64imac",
        };
        let b = CoreContext {
            pc: 0x1000,
            gprs: (0..32).map(|i| (GPR_NAME[i], 0u64)).collect(),
            privilege: 3,
            csrs: vec![(0x344, "mip", !0x82_u64, 0xFF & !0x82)], // same after mask
            word_size: 8,
            isa: "rv64imac",
        };
        assert!(diff_contexts(&a, &b, 1).is_none());
    }

    #[test]
    fn diff_missing_csr_is_mismatch() {
        let a = make_ctx(0x1000, &[0; 32], 3, &[(0x300, 0)]);
        let b = make_ctx(0x1000, &[0; 32], 3, &[]);
        assert!(diff_contexts(&a, &b, 1).is_some());
    }

    #[test]
    fn diff_extra_ref_csr_is_mismatch() {
        let a = make_ctx(0x1000, &[0; 32], 3, &[]);
        let b = make_ctx(0x1000, &[0; 32], 3, &[(0x300, 0)]);
        assert!(diff_contexts(&a, &b, 1).is_some());
    }
}
