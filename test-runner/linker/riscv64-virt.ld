OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;
VIRTUAL_BASE_ADDRESS = 0xffffffc080200000;

SECTIONS
{
    . = VIRTUAL_BASE_ADDRESS;
    __skernel = .;

    __stext = .;
    PROVIDE(__executable_start = .);
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

    .ktest_array : {
        __sktest_array = .;
        KEEP(*(SORT(.ktest_array)))
        __ektest_array = .;
    }

    .cls : {
        __scls = .;
        KEEP(*(.cls .cls.*))
        __ecls = .;
    }

    . = ALIGN(8);
    .eh_frame : {
        PROVIDE(__eh_frame = .);
        KEEP(*(.eh_frame .eh_frame.*))
    }

    .debug_line : {
        KEEP(*(.debug_line))
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        PROVIDE(__heap_start = .);
        *(.bss.heap)
        . += 0x00800000;
        . = ALIGN(4K);
        PROVIDE(__heap_end = .);
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
}
