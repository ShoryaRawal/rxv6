OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY {
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS {
    . = 0x80000000;

    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    } > RAM

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    } > RAM

    .data : ALIGN(4096) {
        *(.sdata .sdata.*)
        *(.data .data.*)
    } > RAM

    .bss : ALIGN(4096) {
        PROVIDE(_bss_start = .);
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        PROVIDE(_bss_end = .);
    } > RAM

    . = ALIGN(16);
    . += 0x10000;
    PROVIDE(_stack_top = .);

    . = ALIGN(4096);
    PROVIDE(_heap_start = .);
    PROVIDE(_heap_end = ORIGIN(RAM) + LENGTH(RAM));
}
