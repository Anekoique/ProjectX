use super::{
    RVCore,
    csr::{CsrAddr, find_desc},
};
use crate::{
    config::Word,
    cpu::debug::{Breakpoint, DebugOps},
    error::{XError, XResult},
    isa::{DECODER, DecodedInst, InstKind, RVReg},
};

#[allow(clippy::unnecessary_cast)] // Word is u32 on isa32, u64 on isa64
fn word_to_u64(w: Word) -> u64 {
    w as u64
}

impl DebugOps for RVCore {
    fn add_breakpoint(&mut self, addr: usize) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.breakpoints.push(Breakpoint { id, addr });
        id
    }

    fn remove_breakpoint(&mut self, id: u32) -> bool {
        self.breakpoints
            .iter()
            .position(|bp| bp.id == id)
            .map(|pos| self.breakpoints.remove(pos))
            .is_some()
    }

    fn list_breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    fn set_skip_bp(&mut self) {
        self.skip_bp_once = true;
    }

    fn context(&self) -> crate::cpu::CoreContext {
        use super::csr::DIFFTEST_CSRS;
        super::context::RVCoreContext {
            pc: self.pc.as_usize() as u64,
            gprs: (0u8..32)
                .map(|i| {
                    (
                        RVReg::from_u8(i).unwrap().name(),
                        word_to_u64(self.gpr[i as usize]),
                    )
                })
                .collect(),
            privilege: self.privilege as u64,
            csrs: DIFFTEST_CSRS
                .iter()
                .map(|&(addr, mask)| {
                    (
                        addr as u16,
                        addr.name(),
                        mask,
                        word_to_u64(self.csr.get(addr)),
                    )
                })
                .collect(),
            word_size: std::mem::size_of::<Word>(),
            isa: if cfg!(isa64) { "rv64imac" } else { "rv32imac" },
        }
    }

    /// Descriptor-aware register read — handles shadow CSRs (sstatus, sie,
    /// sip).
    fn read_register(&self, name: &str) -> Option<u64> {
        match name {
            "pc" => Some(self.pc.as_usize() as u64),
            "privilege" => Some(self.privilege as u64),
            _ => RVReg::from_name(name)
                .map(|r| word_to_u64(self.gpr[r as usize]))
                .or_else(|| {
                    CsrAddr::from_name(name).and_then(|a| {
                        find_desc(a as u16).map(|desc| word_to_u64(self.csr.read_with_desc(desc)))
                    })
                }),
        }
    }

    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64> {
        self.bus
            .lock()
            .map_err(|_| XError::FailedToRead)?
            .read_ram(paddr, size)
            .map(word_to_u64)
    }

    #[allow(clippy::unnecessary_cast)]
    fn fetch_inst(&self, paddr: usize) -> XResult<u32> {
        let bus = self.bus.lock().map_err(|_| XError::FailedToRead)?;
        let lo = bus.read_ram(paddr, 2)? as u32;
        if lo & 0x3 != 0x3 {
            return Ok(lo & 0xFFFF);
        }
        let hi = bus.read_ram(paddr + 2, 2)? as u32;
        Ok(lo | (hi << 16))
    }

    fn disasm_raw(&self, raw: u32) -> String {
        DECODER
            .decode(raw)
            .map(|inst| format_mnemonic(&inst))
            .unwrap_or_else(|_| format!("???  ({raw:#010x})"))
    }
}

/// Format a decoded instruction into a human-readable mnemonic.
pub fn format_mnemonic(inst: &DecodedInst) -> String {
    match inst {
        DecodedInst::R { kind, rd, rs1, rs2 } => {
            format!(
                "{} {}, {}, {}",
                kind.as_str(),
                rd.name(),
                rs1.name(),
                rs2.name()
            )
        }
        DecodedInst::I { kind, rd, rs1, imm } => match kind {
            InstKind::jalr => format!("jalr {}, {}({})", rd.name(), imm, rs1.name()),
            InstKind::lb
            | InstKind::lh
            | InstKind::lw
            | InstKind::ld
            | InstKind::lbu
            | InstKind::lhu
            | InstKind::lwu => {
                format!("{} {}, {}({})", kind.as_str(), rd.name(), imm, rs1.name())
            }
            _ => format!("{} {}, {}, {}", kind.as_str(), rd.name(), rs1.name(), imm),
        },
        DecodedInst::S {
            kind,
            rs1,
            rs2,
            imm,
        } => {
            format!("{} {}, {}({})", kind.as_str(), rs2.name(), imm, rs1.name())
        }
        DecodedInst::B {
            kind,
            rs1,
            rs2,
            imm,
        } => {
            format!("{} {}, {}, {}", kind.as_str(), rs1.name(), rs2.name(), imm)
        }
        DecodedInst::U { kind, rd, imm } => {
            format!(
                "{} {}, {:#x}",
                kind.as_str(),
                rd.name(),
                ((*imm as u32 as u64) >> 12) & 0xFFFFF
            )
        }
        DecodedInst::J { kind, rd, imm } => {
            format!("{} {}, {}", kind.as_str(), rd.name(), imm)
        }
        DecodedInst::C { kind, inst: _ } => kind.as_str().to_string(),
    }
}
