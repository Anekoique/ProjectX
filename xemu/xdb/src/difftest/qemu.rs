//! QEMU difftest backend via GDB Remote Serial Protocol.

use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};

use xcore::CoreContext;

use super::{DiffBackend, gdb::GdbClient};

/// Map CSR address to QEMU GDB register number (4096 + csr_addr).
fn csr_to_qemu_regnum(addr: u16) -> usize {
    4096 + addr as usize
}

fn qemu_bin_for_isa(isa: &str) -> &'static str {
    if isa.starts_with("rv64") {
        "qemu-system-riscv64"
    } else {
        "qemu-system-riscv32"
    }
}

/// QEMU difftest backend via GDB RSP.
pub struct QemuBackend {
    proc: Child,
    gdb: GdbClient,
    gpr_names: Vec<&'static str>,
    csr_meta: Vec<(u16, &'static str, u64)>, // (addr, name, mask)
    word_size: usize,
    isa: &'static str,
}

impl QemuBackend {
    /// Spawn QEMU, connect via GDB, and sync initial DUT state.
    pub fn new(
        binary_path: &str,
        reset_vec: usize,
        init_ctx: &CoreContext,
    ) -> Result<Self, String> {
        let qemu_bin = qemu_bin_for_isa(init_ctx.isa);

        // Verify QEMU exists
        Command::new("which")
            .arg(qemu_bin)
            .output()
            .map_err(|e| format!("{e}"))
            .and_then(|o| {
                o.status
                    .success()
                    .then_some(())
                    .ok_or(format!("{qemu_bin} not found in PATH"))
            })?;

        // Spawn QEMU with GDB stub.
        // Use -bios <binary> so QEMU loads it at the reset vector.
        let proc = Command::new(qemu_bin)
            .args([
                "-M",
                "virt",
                "-m",
                "256M",
                "-nographic",
                "-s",
                "-S",
                "-bios",
                binary_path,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn {qemu_bin}: {e}"))?;

        std::thread::sleep(Duration::from_millis(300));

        // Connect GDB
        let mut gdb = GdbClient::connect("127.0.0.1:1234")?;

        // Configure QEMU features (require QEMU 7.0+)
        let require_qemu_cmd =
            |gdb: &mut GdbClient, cmd: &str, feature: &str| -> Result<(), String> {
                let resp = gdb.send_recv(cmd)?;
                (resp.starts_with(b"OK"))
                    .then_some(())
                    .ok_or(format!("QEMU {feature} not supported. Requires QEMU 7.0+."))
            };
        // Suppress interrupts during single-step (NOIRQ+NOTIMER)
        require_qemu_cmd(&mut gdb, "Qqemu.sstep=0x7", "sstep")?;
        // Physical memory mode for direct address access
        require_qemu_cmd(&mut gdb, "Qqemu.PhyMemMode:1", "PhyMemMode")?;

        // QEMU virt machine starts at ROM (0x1000) which initializes registers
        // and jumps to 0x80000000. Run to reset vector first, then sync DUT state.
        gdb.set_breakpoint(reset_vec)?;
        gdb.cont()?;
        gdb.remove_breakpoint(reset_vec)?;

        let gpr_names: Vec<&'static str> = init_ctx.gprs.iter().map(|(n, _)| *n).collect();
        let csr_meta: Vec<(u16, &'static str, u64)> = init_ctx
            .csrs
            .iter()
            .map(|&(a, n, m, _)| (a, n, m))
            .collect();

        let mut backend = Self {
            proc,
            gdb,
            gpr_names,
            csr_meta,
            word_size: init_ctx.word_size,
            isa: init_ctx.isa,
        };

        // Sync initial DUT state to REF — overwrite QEMU's firmware-set registers
        backend
            .sync_state(init_ctx)
            .map_err(|e| format!("initial sync: {e}"))?;

        info!("difftest: QEMU attached (pid {})", backend.proc.id());
        Ok(backend)
    }
}

impl DiffBackend for QemuBackend {
    fn step(&mut self) -> Result<(), String> {
        self.gdb.step()
    }

    fn read_context(&mut self) -> Result<CoreContext, String> {
        let regs = self.gdb.read_regs()?;
        Ok(CoreContext {
            pc: regs[32],
            gprs: self
                .gpr_names
                .iter()
                .enumerate()
                .map(|(i, &name)| (name, regs[i]))
                .collect(),
            privilege: 0, // privilege checked via mstatus MPP/SPP in CSR whitelist
            csrs: self
                .csr_meta
                .iter()
                .map(|&(addr, name, mask)| {
                    let raw = self
                        .gdb
                        .read_register(csr_to_qemu_regnum(addr), self.word_size)
                        .unwrap_or(0);
                    (addr, name, mask, raw)
                })
                .collect(),
            word_size: self.word_size,
            isa: self.isa,
        })
    }

    fn sync_state(&mut self, ctx: &CoreContext) -> Result<(), String> {
        // Write all GPRs + PC via bulk 'G' command
        let mut regs: Vec<u64> = ctx.gprs.iter().map(|(_, v)| *v).collect();
        regs.push(ctx.pc);
        self.gdb.write_regs(&regs, self.word_size)?;
        // Write raw CSR values (not masked)
        for &(addr, _, _, raw) in &ctx.csrs {
            self.gdb
                .write_register(csr_to_qemu_regnum(addr), raw, self.word_size)?;
        }
        Ok(())
    }

    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> {
        self.gdb.write_mem(addr, data)
    }

    fn name(&self) -> &str {
        "qemu"
    }
}

impl Drop for QemuBackend {
    fn drop(&mut self) {
        let _ = self.proc.kill();
        let _ = self.proc.wait();
    }
}
