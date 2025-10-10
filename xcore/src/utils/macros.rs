#[macro_export]
macro_rules! import_modules {
    ($($arch:ident => $mod:ident),* $(,)?) => {
        $(
            #[cfg($arch)]
            mod $mod;
        )*
    };
    ($($arch:ident),* $(,)?) => {
        $(
            #[cfg($arch)]
            mod $arch;
        )*
    };
}

#[macro_export]
macro_rules! define_cpu {
    ($($arch:ident => $core_type:ty),* $(,)?) => {
        $(
            #[cfg($arch)]
            pub static XCPU: std::sync::LazyLock<std::sync::Mutex<CPU<$core_type>>> =
                std::sync::LazyLock::new(|| {
                    std::sync::Mutex::new(CPU::new(<$core_type>::new()))
                });
        )*
    };
}

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
