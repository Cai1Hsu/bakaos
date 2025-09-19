OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;
VIRTUAL_BASE_ADDRESS = 0xffffffc080200000;

SECTIONS
{
    . = VIRTUAL_BASE_ADDRESS;
    __skernel = .;

    __stext = .;
    .text ALIGN(4K) : AT(BASE_ADDRESS) {
        KEEP(*(.text.entry))
        *(.text .text.*)
        . = ALIGN(4K);
    }

    . = ALIGN(4K);
    __etext = .;
    __srodata = .;
    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    __erodata = .;
    __sdata = .;
    .data ALIGN(4K) : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .cls : {
        __scls = .;
        *(.cls .cls.*)
        __ecls = .;
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        *(.bss.heap)
        . += 0x00800000;
        *(.bss.stack)
        PROVIDE(__stack_bottom = .);
        . = ALIGN(4K);
        . += 0x00180000;
        . = ALIGN(4K);
        PROVIDE(__stack_top = .);
        __sbss = .;
        *(.bss .bss.*)
    }

    . = ALIGN(4K);
    __ebss = .;
    __ekernel = .;

    .debug_line : {
        KEEP(*(.debug_line))
    }
}
