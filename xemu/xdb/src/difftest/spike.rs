//! Spike difftest backend via FFI to the C++ wrapper.
//! Requires Spike installed (SPIKE_DIR defaults to /opt/homebrew).

use xcore::CoreContext;

use super::DiffBackend;

// ── FFI bindings ──

mod ffi {
    use std::os::raw::c_char;

    #[repr(C)]
    pub struct SpikeMemRegion {
        pub base: usize,
        pub size: usize,
    }

    pub enum SpikeCtx {}

    unsafe extern "C" {
        pub fn spike_init(
            regions: *const SpikeMemRegion,
            n: usize,
            init_pc: u64,
            xlen: u32,
            isa: *const c_char,
        ) -> *mut SpikeCtx;
        pub fn spike_fini(ctx: *mut SpikeCtx);
        pub fn spike_step(ctx: *mut SpikeCtx) -> i32;
        pub fn spike_get_pc(ctx: *mut SpikeCtx, out: *mut u64);
        pub fn spike_get_gpr(ctx: *mut SpikeCtx, out: *mut u64);
        pub fn spike_get_csr(ctx: *mut SpikeCtx, addr: u16) -> u64;
        pub fn spike_get_priv(ctx: *mut SpikeCtx) -> u64;
        pub fn spike_set_pc(ctx: *mut SpikeCtx, pc: u64);
        pub fn spike_set_gpr(ctx: *mut SpikeCtx, gpr: *const u64);
        pub fn spike_set_csr(ctx: *mut SpikeCtx, addr: u16, val: u64);
        pub fn spike_copy_mem(ctx: *mut SpikeCtx, addr: usize, data: *const u8, len: usize);
        pub fn spike_write_mem(ctx: *mut SpikeCtx, addr: usize, data: *const u8, len: usize)
        -> i32;
    }
}

// ── Backend ──

pub struct SpikeBackend {
    ctx: *mut ffi::SpikeCtx,
    gpr_names: Vec<&'static str>,
    csr_meta: Vec<(u16, &'static str, u64)>,
    word_size: usize,
    isa: &'static str,
}

impl SpikeBackend {
    pub fn new(
        binary_path: &str,
        reset_vec: usize,
        init_ctx: &CoreContext,
    ) -> Result<Self, String> {
        let region = ffi::SpikeMemRegion {
            base: 0x8000_0000,
            size: 256 * 1024 * 1024,
        };
        let xlen = (init_ctx.word_size * 8) as u32;
        let isa_c = std::ffi::CString::new(init_ctx.isa).map_err(|e| format!("ISA string: {e}"))?;

        let ctx = unsafe { ffi::spike_init(&region, 1, reset_vec as u64, xlen, isa_c.as_ptr()) };
        if ctx.is_null() {
            return Err("Spike init failed".into());
        }

        // Load binary into Spike memory
        let bytes = std::fs::read(binary_path).map_err(|e| format!("read binary: {e}"))?;
        unsafe {
            ffi::spike_copy_mem(ctx, reset_vec, bytes.as_ptr(), bytes.len());
        }

        // Sync initial state from DUT
        let gpr_vals: Vec<u64> = init_ctx.gprs.iter().map(|(_, v)| *v).collect();
        unsafe {
            ffi::spike_set_pc(ctx, init_ctx.pc);
            ffi::spike_set_gpr(ctx, gpr_vals.as_ptr());
            for &(addr, _, _, raw) in &init_ctx.csrs {
                ffi::spike_set_csr(ctx, addr, raw);
            }
        }

        let gpr_names = init_ctx.gprs.iter().map(|(n, _)| *n).collect();
        let csr_meta = init_ctx
            .csrs
            .iter()
            .map(|&(a, n, m, _)| (a, n, m))
            .collect();

        info!("difftest: Spike attached");
        Ok(Self {
            ctx,
            gpr_names,
            csr_meta,
            word_size: init_ctx.word_size,
            isa: init_ctx.isa,
        })
    }
}

impl DiffBackend for SpikeBackend {
    fn step(&mut self) -> Result<(), String> {
        let ret = unsafe { ffi::spike_step(self.ctx) };
        (ret == 0 || ret == 1)
            .then_some(())
            .ok_or_else(|| "Spike step error".into())
    }

    fn read_context(&mut self) -> Result<CoreContext, String> {
        let (mut pc, mut gpr_vals) = (0u64, [0u64; 32]);
        unsafe {
            ffi::spike_get_pc(self.ctx, &mut pc);
            ffi::spike_get_gpr(self.ctx, gpr_vals.as_mut_ptr());
        }
        Ok(CoreContext {
            pc,
            gprs: self
                .gpr_names
                .iter()
                .enumerate()
                .map(|(i, &name)| (name, gpr_vals[i]))
                .collect(),
            privilege: unsafe { ffi::spike_get_priv(self.ctx) },
            csrs: self
                .csr_meta
                .iter()
                .map(|&(addr, name, mask)| {
                    (addr, name, mask, unsafe {
                        ffi::spike_get_csr(self.ctx, addr)
                    })
                })
                .collect(),
            word_size: self.word_size,
            isa: self.isa,
        })
    }

    fn sync_state(&mut self, ctx: &CoreContext) -> Result<(), String> {
        let gpr_vals: Vec<u64> = ctx.gprs.iter().map(|(_, v)| *v).collect();
        unsafe {
            ffi::spike_set_pc(self.ctx, ctx.pc);
            ffi::spike_set_gpr(self.ctx, gpr_vals.as_ptr());
            for &(addr, _, _, raw) in &ctx.csrs {
                ffi::spike_set_csr(self.ctx, addr, raw);
            }
        }
        Ok(())
    }

    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> {
        let ret = unsafe { ffi::spike_write_mem(self.ctx, addr, data.as_ptr(), data.len()) };
        (ret == 0)
            .then_some(())
            .ok_or_else(|| "Spike write_mem failed".into())
    }

    fn name(&self) -> &str {
        "spike"
    }
}

impl Drop for SpikeBackend {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            unsafe { ffi::spike_fini(self.ctx) };
        }
    }
}
