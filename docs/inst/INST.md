# Instruction Set Reference

Supported extensions: **RV32I/RV64I** base, **M** (multiply/divide), **A** (atomic), **Zicsr**, **C** (compressed), **Privileged**.

ISA width is compile-time: `cfg(isa32)` for RV32, `cfg(isa64)` for RV64. RV64-only instructions return `InvalidInst` on RV32.

---

## Encoding Formats

| Format | Fields |
|--------|--------|
| R | `funct7 rs2 rs1 funct3 rd opcode` |
| I | `imm[11:0] rs1 funct3 rd opcode` |
| S | `imm[11:5] rs2 rs1 funct3 imm[4:0] opcode` |
| B | `imm[12\|10:5] rs2 rs1 funct3 imm[4:1\|11] opcode` |
| U | `imm[31:12] rd opcode` |
| J | `imm[20\|10:1\|11\|19:12] rd opcode` |

Compressed formats (16-bit): CR, CI, CSS, CIW, CL, CS, CA, CB, CJ.

---

## RV32I / RV64I Base

### Arithmetic & Logic (R-type, opcode `0110011`)

| Mnemonic | funct7 | funct3 | Description |
|----------|--------|--------|-------------|
| `add`  | 0000000 | 000 | rd = rs1 + rs2 |
| `sub`  | 0100000 | 000 | rd = rs1 - rs2 |
| `sll`  | 0000000 | 001 | rd = rs1 << rs2[4:0] |
| `slt`  | 0000000 | 010 | rd = (rs1 <s rs2) ? 1 : 0 |
| `sltu` | 0000000 | 011 | rd = (rs1 <u rs2) ? 1 : 0 |
| `xor`  | 0000000 | 100 | rd = rs1 ^ rs2 |
| `srl`  | 0000000 | 101 | rd = rs1 >>u rs2[4:0] |
| `sra`  | 0100000 | 101 | rd = rs1 >>s rs2[4:0] |
| `or`   | 0000000 | 110 | rd = rs1 \| rs2 |
| `and`  | 0000000 | 111 | rd = rs1 & rs2 |

### RV64I Word-width (R-type, opcode `0111011`)

| Mnemonic | funct7 | funct3 | Description |
|----------|--------|--------|-------------|
| `addw` | 0000000 | 000 | rd = sext32(rs1 + rs2) |
| `subw` | 0100000 | 000 | rd = sext32(rs1 - rs2) |
| `sllw` | 0000000 | 001 | rd = sext32(rs1[31:0] << rs2[4:0]) |
| `srlw` | 0000000 | 101 | rd = sext32(rs1[31:0] >>u rs2[4:0]) |
| `sraw` | 0100000 | 101 | rd = sext32(rs1[31:0] >>s rs2[4:0]) |

### Immediate (I-type, opcode `0010011`)

| Mnemonic | funct3 | Description |
|----------|--------|-------------|
| `addi`  | 000 | rd = rs1 + sext(imm) |
| `slti`  | 010 | rd = (rs1 <s sext(imm)) ? 1 : 0 |
| `sltiu` | 011 | rd = (rs1 <u sext(imm)) ? 1 : 0 |
| `xori`  | 100 | rd = rs1 ^ sext(imm) |
| `ori`   | 110 | rd = rs1 \| sext(imm) |
| `andi`  | 111 | rd = rs1 & sext(imm) |
| `slli`  | 001 | rd = rs1 << imm[5:0] |
| `srli`  | 101 | rd = rs1 >>u imm[5:0] |
| `srai`  | 101 | rd = rs1 >>s imm[5:0] (funct7 bit 5 = 1) |

### RV64I Immediate Word-width (I-type, opcode `0011011`)

| Mnemonic | funct3 | Description |
|----------|--------|-------------|
| `addiw` | 000 | rd = sext32(rs1 + sext(imm)) |
| `slliw` | 001 | rd = sext32(rs1[31:0] << imm[4:0]) |
| `srliw` | 101 | rd = sext32(rs1[31:0] >>u imm[4:0]) |
| `sraiw` | 101 | rd = sext32(rs1[31:0] >>s imm[4:0]) |

### Load (I-type, opcode `0000011`)

| Mnemonic | funct3 | Description |
|----------|--------|-------------|
| `lb`  | 000 | rd = sext8(M[rs1 + imm]) |
| `lh`  | 001 | rd = sext16(M[rs1 + imm]) |
| `lw`  | 010 | rd = sext32(M[rs1 + imm]) |
| `ld`  | 011 | rd = M[rs1 + imm] (RV64) |
| `lbu` | 100 | rd = zext8(M[rs1 + imm]) |
| `lhu` | 101 | rd = zext16(M[rs1 + imm]) |
| `lwu` | 110 | rd = zext32(M[rs1 + imm]) (RV64) |

### Store (S-type, opcode `0100011`)

| Mnemonic | funct3 | Description |
|----------|--------|-------------|
| `sb` | 000 | M[rs1 + imm] = rs2[7:0] |
| `sh` | 001 | M[rs1 + imm] = rs2[15:0] |
| `sw` | 010 | M[rs1 + imm] = rs2[31:0] |
| `sd` | 011 | M[rs1 + imm] = rs2 (RV64) |

### Branch (B-type, opcode `1100011`)

| Mnemonic | funct3 | Description |
|----------|--------|-------------|
| `beq`  | 000 | if rs1 == rs2 then PC += imm |
| `bne`  | 001 | if rs1 != rs2 then PC += imm |
| `blt`  | 100 | if rs1 <s rs2 then PC += imm |
| `bge`  | 101 | if rs1 >=s rs2 then PC += imm |
| `bltu` | 110 | if rs1 <u rs2 then PC += imm |
| `bgeu` | 111 | if rs1 >=u rs2 then PC += imm |

### Jump & Upper Immediate

| Mnemonic | Format | Opcode | Description |
|----------|--------|--------|-------------|
| `jal`   | J | `1101111` | rd = PC+4; PC += imm |
| `jalr`  | I | `1100111` | rd = PC+4; PC = (rs1 + imm) & ~1 |
| `lui`   | U | `0110111` | rd = imm << 12 |
| `auipc` | U | `0010111` | rd = PC + (imm << 12) |

---

## M Extension (Multiply/Divide)

All R-type, opcode `0110011`, funct7 `0000001`.

| Mnemonic | funct3 | Description |
|----------|--------|-------------|
| `mul`    | 000 | rd = (rs1 * rs2)[XLEN-1:0] |
| `mulh`   | 001 | rd = (rs1 *s rs2)[2*XLEN-1:XLEN] |
| `mulhsu` | 010 | rd = (rs1 *s rs2 *u)[2*XLEN-1:XLEN] |
| `mulhu`  | 011 | rd = (rs1 *u rs2)[2*XLEN-1:XLEN] |
| `div`    | 100 | rd = rs1 /s rs2 |
| `divu`   | 101 | rd = rs1 /u rs2 |
| `rem`    | 110 | rd = rs1 %s rs2 |
| `remu`   | 111 | rd = rs1 %u rs2 |

### RV64M Word-width (opcode `0111011`, funct7 `0000001`)

| Mnemonic | funct3 | Description |
|----------|--------|-------------|
| `mulw`  | 000 | rd = sext32(rs1[31:0] * rs2[31:0]) |
| `divw`  | 100 | rd = sext32(rs1[31:0] /s rs2[31:0]) |
| `divuw` | 101 | rd = sext32(rs1[31:0] /u rs2[31:0]) |
| `remw`  | 110 | rd = sext32(rs1[31:0] %s rs2[31:0]) |
| `remuw` | 111 | rd = sext32(rs1[31:0] %u rs2[31:0]) |

Division edge cases: div-by-zero returns `MAX` (unsigned) or `-1` (signed word); signed overflow (`MIN / -1`) returns `MIN`.

---

## A Extension (Atomic)

All R-type, opcode `0101111`. Bits [26:25] encode `aq`/`rl` ordering hints (ignored on single-hart). Naming uses underscore: `lr_w`, `amoadd_d`, etc.

### RV32A (funct3 = `010`, word)

| Mnemonic | funct5 | Description |
|----------|--------|-------------|
| `lr_w`       | 00010 | rd = sext32(M[rs1]); reserve addr |
| `sc_w`       | 00011 | if reserved(rs1) { M[rs1] = rs2; rd = 0 } else { rd = 1 }; clear |
| `amoswap_w`  | 00001 | rd = sext32(M[rs1]); M[rs1] = rs2 |
| `amoadd_w`   | 00000 | rd = sext32(M[rs1]); M[rs1] = rd + rs2 |
| `amoxor_w`   | 00100 | rd = sext32(M[rs1]); M[rs1] = rd ^ rs2 |
| `amoand_w`   | 01100 | rd = sext32(M[rs1]); M[rs1] = rd & rs2 |
| `amoor_w`    | 01000 | rd = sext32(M[rs1]); M[rs1] = rd \| rs2 |
| `amomin_w`   | 10000 | rd = sext32(M[rs1]); M[rs1] = min_s(rd, rs2) |
| `amomax_w`   | 10100 | rd = sext32(M[rs1]); M[rs1] = max_s(rd, rs2) |
| `amominu_w`  | 11000 | rd = sext32(M[rs1]); M[rs1] = min_u(rd, rs2) |
| `amomaxu_w`  | 11100 | rd = sext32(M[rs1]); M[rs1] = max_u(rd, rs2) |

### RV64A (funct3 = `011`, doubleword)

| Mnemonic | funct5 | Description |
|----------|--------|-------------|
| `lr_d`       | 00010 | rd = M[rs1]; reserve addr |
| `sc_d`       | 00011 | if reserved(rs1) { M[rs1] = rs2; rd = 0 } else { rd = 1 }; clear |
| `amoswap_d`  | 00001 | rd = M[rs1]; M[rs1] = rs2 |
| `amoadd_d`   | 00000 | rd = M[rs1]; M[rs1] = rd + rs2 |
| `amoxor_d`   | 00100 | rd = M[rs1]; M[rs1] = rd ^ rs2 |
| `amoand_d`   | 01100 | rd = M[rs1]; M[rs1] = rd & rs2 |
| `amoor_d`    | 01000 | rd = M[rs1]; M[rs1] = rd \| rs2 |
| `amomin_d`   | 10000 | rd = M[rs1]; M[rs1] = min_s(rd, rs2) |
| `amomax_d`   | 10100 | rd = M[rs1]; M[rs1] = max_s(rd, rs2) |
| `amominu_d`  | 11000 | rd = M[rs1]; M[rs1] = min_u(rd, rs2) |
| `amomaxu_d`  | 11100 | rd = M[rs1]; M[rs1] = max_u(rd, rs2) |

### Implementation Notes

- **LR/SC reservation**: `Option<usize>` on `RVCore` tracks the reserved physical address. SC always clears the reservation regardless of success/failure.
- **aq/rl bits**: Parsed via wildcard (`??`) in instpat; no-op on single-hart.
- **AMO helper**: `amo_op` encapsulates load→compute→store. Each AMO instruction is a one-liner closure.
- **Sign-extension**: `.w` variants sign-extend the loaded 32-bit value to XLEN via `as i32 as i64 as Word`.

---

## Zicsr Extension

All I-type, opcode `1110011`. CSR address is encoded in imm[11:0].

| Mnemonic | funct3 | Description |
|----------|--------|-------------|
| `csrrw`  | 001 | rd = CSR; CSR = rs1 |
| `csrrs`  | 010 | rd = CSR; CSR \|= rs1 |
| `csrrc`  | 011 | rd = CSR; CSR &= ~rs1 |
| `csrrwi` | 101 | rd = CSR; CSR = zext(uimm) |
| `csrrsi` | 110 | rd = CSR; CSR \|= zext(uimm) |
| `csrrci` | 111 | rd = CSR; CSR &= ~zext(uimm) |

Optimization: `csrrw` with rd=x0 skips read; `csrrs`/`csrrc` with rs1=x0 skip write.

---

## Privileged Instructions

| Mnemonic | Encoding | Description |
|----------|----------|-------------|
| `ecall`  | `000000000000 00000 000 00000 1110011` | Environment call (trap based on current privilege) |
| `ebreak` | `000000000001 00000 000 00000 1110011` | Breakpoint trap |
| `mret`   | `0011000 00010 00000 000 00000 1110011` | Return from M-mode trap |
| `sret`   | `0001000 00010 00000 000 00000 1110011` | Return from S-mode trap |

---

## C Extension (Compressed, 16-bit)

### Quadrant 0 (bits [1:0] = `00`)

| Mnemonic | Format | Description |
|----------|--------|-------------|
| `c.addi4spn` | CIW | rd' = sp + zext(imm*4) |
| `c.lw`       | CL  | rd' = sext32(M[rs1' + imm]) |
| `c.ld`       | CL  | rd' = M[rs1' + imm] (RV64) |
| `c.sw`       | CS  | M[rs1' + imm] = rs2'[31:0] |
| `c.sd`       | CS  | M[rs1' + imm] = rs2' (RV64) |

### Quadrant 1 (bits [1:0] = `01`)

| Mnemonic | Format | Description |
|----------|--------|-------------|
| `c.nop`      | CI | No operation |
| `c.addi`     | CI | rd = rd + sext(imm) |
| `c.addiw`    | CI | rd = sext32(rd + sext(imm)) (RV64) |
| `c.li`       | CI | rd = sext(imm) |
| `c.addi16sp` | CI | sp = sp + sext(imm*16) |
| `c.lui`      | CI | rd = sext(imm << 12) |
| `c.srli`     | CB | rd' = rd' >>u imm |
| `c.srai`     | CB | rd' = rd' >>s imm |
| `c.andi`     | CB | rd' = rd' & sext(imm) |
| `c.sub`      | CA | rd' = rd' - rs2' |
| `c.xor`      | CA | rd' = rd' ^ rs2' |
| `c.or`       | CA | rd' = rd' \| rs2' |
| `c.and`      | CA | rd' = rd' & rs2' |
| `c.subw`     | CA | rd' = sext32(rd' - rs2') (RV64) |
| `c.addw`     | CA | rd' = sext32(rd' + rs2') (RV64) |
| `c.j`        | CJ | PC += sext(imm) |
| `c.beqz`     | CB | if rd' == 0 then PC += sext(imm) |
| `c.bnez`     | CB | if rd' != 0 then PC += sext(imm) |

### Quadrant 2 (bits [1:0] = `10`)

| Mnemonic | Format | Description |
|----------|--------|-------------|
| `c.slli`  | CI  | rd = rd << imm |
| `c.lwsp`  | CI  | rd = sext32(M[sp + imm]) |
| `c.ldsp`  | CI  | rd = M[sp + imm] (RV64) |
| `c.jr`    | CR  | PC = rs1 |
| `c.mv`    | CR  | rd = rs2 |
| `c.ebreak`| CR  | Breakpoint trap |
| `c.jalr`  | CR  | ra = PC+2; PC = rs1 |
| `c.add`   | CR  | rd = rd + rs2 |
| `c.swsp`  | CSS | M[sp + imm] = rs2[31:0] |
| `c.sdsp`  | CSS | M[sp + imm] = rs2 (RV64) |

---

## Notation

- `<s` / `<u` — signed / unsigned comparison
- `>>s` / `>>u` — arithmetic / logical shift right
- `sext32(x)` — sign-extend 32-bit result to XLEN
- `rd'`, `rs1'`, `rs2'` — compressed register set (x8-x15)
- `M[addr]` — memory at address
