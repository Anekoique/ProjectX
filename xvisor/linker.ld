OUTPUT_ARCH(riscv)

BASE_ADDRESS = 0x80200000;

ENTRY(_start)

SECTIONS
{
    . = BASE_ADDRESS;
    _hyp_start = .;

    .text : ALIGN(4K) {
        *(.text.boot)
        *(.text .text.*)
        . = ALIGN(4K);
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        . = ALIGN(4K);
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
        *(.sdata .sdata.*)
        . = ALIGN(4K);
    }

    .bss : ALIGN(4K) {
        _stack_start = .;
        KEEP(*(.bss.stack))
        . = ALIGN(4K);
        _stack_end = .;

        _bss_start = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4K);
        _bss_end = .;
    }

    _hyp_end = .;

    /DISCARD/ : {
        *(.comment) *(.gnu*) *(.note*) *(.eh_frame*)
    }
}
