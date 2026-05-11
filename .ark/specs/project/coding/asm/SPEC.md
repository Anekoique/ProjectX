# Assembly Guidelines

> Rules for assembly in module-level `global_asm!` blocks and standalone `.S` files across the workspace.

## R1 — Use the short directive for built-in sections; `.section` with flags and type for custom

**Applies to:** `**/*.S`, `global_asm!` blocks
**Evidence:** VERIFY

For built-in sections use the short directive (`.text`). For custom sections use `.section` with explicit flags and type. A blank line follows each section definition to separate it from the code below.

```asm
# Bad — custom section without flags/type
.section ".bsp_boot.stack"
boot_stack_bottom:
    .skip 0x40000

# Good
.section ".bsp_boot.stack", "aw", @nobits

boot_stack_bottom:
    .balign 4096
    .skip 0x40000  # 256 KiB
boot_stack_top:
```

## R2 — Code-width directive sits directly after the section definition

**Applies to:** `**/*.S`, `global_asm!` blocks (x86-64)
**Evidence:** VERIFY

For x86-64, when an executable section contains only 64-bit code, place `.code64` directly after the section definition. Same for `.code32`. In mixed sections, treat `.code64`/`.code32` as function attributes.

```asm
# Bad — code width is implicit
.text
.global foo
foo:
    mov rax, 1
    ret

# Good
.text
.code64

.global foo
foo:
    mov rax, 1
    ret
```

## R3 — Function attributes sit directly above the label, unindented

**Applies to:** `**/*.S`, `global_asm!` blocks
**Evidence:** VERIFY

`.global`, `.balign`, `.type` belong directly before the function label and are not indented. Prefer `.global` over `.globl`.

```asm
# Bad — indented attributes; .globl form
    .globl foo
    .balign 4
foo:
    ret

# Good
.balign 4
.global foo
foo:
    ret
```

## R4 — Functions callable from Rust declare `.type` and `.size`

**Applies to:** `**/*.S`, `global_asm!` blocks
**Evidence:** VERIFY

`.type` and `.size` give debuggers a complete picture of the function. Boot entry points, exception trampolines, and interrupt trampolines are exempt — they may not fit the typical "function" definition and their sizes can be ill-defined.

```asm
# Bad — Rust-callable function without type/size
.global bar
bar:
    mov rax, 2
    ret

# Good
.global bar
.type bar, @function
bar:
    mov rax, 2
    ret
.size bar, .-bar
```

## R5 — `global_asm!` labels carry a unique prefix to avoid name clashes

**Applies to:** `global_asm!` blocks
**Evidence:** VERIFY

A Rust crate is a single translation unit, so `global_asm!` labels in different modules within the same crate share the same global namespace. Add a prefix that names the module's role (`bsp_`, `ap_`, `trap_`, `irq_`).

```asm
# Bad — generic label clashes across modules
boot_stack_top:

# Good
bsp_boot_stack_top:
ap_boot_stack_top:
```

## R6 — Use `.balign` for alignment, never `.align`

**Applies to:** `**/*.S`, `global_asm!` blocks
**Evidence:** grep

`.align`'s behavior varies across architectures — on some it specifies a byte count, on others a power of two. `.balign` is unambiguous byte-count alignment.

```asm
# Bad — architecture-dependent meaning
.align 12

# Good
.balign 4096
```
