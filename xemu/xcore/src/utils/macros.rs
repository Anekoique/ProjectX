#[macro_export]
macro_rules! rv_inst_table {
    ($macro:ident) => {
        $macro! {
            (R, (rd, rs1, rs2), [add, addw, sub, subw, sll, sllw, slt, sltu, xor, srl, srlw, sra, sraw, or, and, mul, mulw, mulh, mulhsu, mulhu, div, divw, divu, divuw, rem, remw, remu, remuw, mret]),
            (I, (rd, rs1, imm), [addi, addiw, slli, slliw, slti, sltiu, xori, srli, srliw, srai, sraiw, ori, andi, lb, lh, lw, ld, lbu, lhu, lwu, jalr, csrrw, csrrs, csrrc, csrrwi, csrrsi, csrrci, ebreak, ecall]),
            (S, (rs1, rs2, imm), [sb, sh, sw, sd]),
            (B, (rs1, rs2, imm), [beq, bne, blt, bge, bltu, bgeu]),
            (U, (rd, imm), [lui, auipc]),
            (J, (rd, imm), [jal]),
            (C, (inst), [c_jr, c_mv, c_ebreak, c_jalr, c_add, c_nop, c_addi, c_addiw, c_li, c_addi16sp, c_lui, c_slli, c_lwsp, c_ldsp, c_swsp, c_sdsp, c_addi4spn, c_lw, c_ld, c_sw, c_sd, c_sub, c_xor, c_or, c_and, c_subw, c_addw, c_srli, c_srai, c_andi, c_beqz, c_bnez, c_j]),
        }
    };
}
