#[macro_export]
macro_rules! rv_inst_table {
    ($macro:ident) => {
        $macro! {
            (R, (rd, rs1, rs2), [add, sub, sll, slt, sltu, xor, srl, sra, or, and, mul, mulh, mulhu, div, divu, rem, remu, mret]),
            (I, (rd, rs1, imm), [addi, slli, slti, sltiu, xori, srli, srla, ori, andi, lb, lh, lw, lbu, lhu, jalr, csrrw, csrrs, csrrc, csrrwi, csrrsi, csrrci, ebreak, ecall]),
            (S, (rs1, rs2, imm), [sb, sh, sw]),
            (B, (rs1, rs2, imm), [beq, bne, blt, bge, bltu, bgeu]),
            (U, (rd, imm), [lui, auipc]),
            (J, (rd, imm), [jal])
        }
    };
}
