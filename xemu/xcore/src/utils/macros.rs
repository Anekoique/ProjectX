#[macro_export]
macro_rules! rv_inst_table {
    ($macro:ident) => {
        $macro! {
            (R, (rd, rs1, rs2), [add, addw, sub, subw, sll, sllw, slt, sltu, xor, srl, srlw, sra, sraw, or, and, mul, mulw, mulh, mulhsu, mulhu, div, divw, divu, divuw, rem, remw, remu, remuw, mret, sret, sfence_vma, lr_w, sc_w, amoswap_w, amoadd_w, amoxor_w, amoand_w, amoor_w, amomin_w, amomax_w, amominu_w, amomaxu_w, lr_d, sc_d, amoswap_d, amoadd_d, amoxor_d, amoand_d, amoor_d, amomin_d, amomax_d, amominu_d, amomaxu_d]),
            (FR, (rd, rs1, rs2, rm), [fadd_s, fsub_s, fmul_s, fdiv_s, fsqrt_s, fsgnj_s, fsgnjn_s, fsgnjx_s, fmin_s, fmax_s, fcvt_w_s, fcvt_wu_s, fmv_x_w, feq_s, flt_s, fle_s, fclass_s, fcvt_s_w, fcvt_s_wu, fmv_w_x, fcvt_l_s, fcvt_lu_s, fcvt_s_l, fcvt_s_lu, fadd_d, fsub_d, fmul_d, fdiv_d, fsqrt_d, fsgnj_d, fsgnjn_d, fsgnjx_d, fmin_d, fmax_d, fcvt_s_d, fcvt_d_s, feq_d, flt_d, fle_d, fclass_d, fcvt_w_d, fcvt_wu_d, fcvt_d_w, fcvt_d_wu, fcvt_l_d, fcvt_lu_d, fmv_x_d, fcvt_d_l, fcvt_d_lu, fmv_d_x]),
            (FR4, (rd, rs1, rs2, rs3, rm), [fmadd_s, fmsub_s, fnmsub_s, fnmadd_s, fmadd_d, fmsub_d, fnmsub_d, fnmadd_d]),
            (I, (rd, rs1, imm), [addi, addiw, slli, slliw, slti, sltiu, xori, srli, srliw, srai, sraiw, ori, andi, lb, lh, lw, ld, lbu, lhu, lwu, jalr, csrrw, csrrs, csrrc, csrrwi, csrrsi, csrrci, ebreak, ecall, fence, fence_i, wfi, flw, fld]),
            (S, (rs1, rs2, imm), [sb, sh, sw, sd, fsw, fsd]),
            (B, (rs1, rs2, imm), [beq, bne, blt, bge, bltu, bgeu]),
            (U, (rd, imm), [lui, auipc]),
            (J, (rd, imm), [jal]),
            (C, (inst), [c_jr, c_mv, c_ebreak, c_jalr, c_add, c_nop, c_addi, c_addiw, c_li, c_addi16sp, c_lui, c_slli, c_lwsp, c_ldsp, c_swsp, c_sdsp, c_addi4spn, c_lw, c_ld, c_sw, c_sd, c_sub, c_xor, c_or, c_and, c_subw, c_addw, c_srli, c_srai, c_andi, c_beqz, c_bnez, c_j, c_fld, c_fsd, c_fldsp, c_fsdsp]),
        }
    };
}
