OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

PHDRS
{
    load PT_LOAD FLAGS(0x7);   /* RWX */
}

KERNEL_BASE = 0xffffffff80000000;
KERNEL_PHYS_OFFSET = 0x100000; /* 1 MiB offset */

SECTIONS
{
    . = KERNEL_BASE + KERNEL_PHYS_OFFSET;

    .text : ALIGN(4K)
    {
        *(.text.boot)
        *(.text .text.*)
    } :load

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } :load

    .data : ALIGN(4K)
    {
        *(.data .data.*)
        *(.got .got.*)
    } :load

    .requests.start : ALIGN(8)
    {
        KEEP(*(.requests_start))
    } :load

    .requests : ALIGN(8)
    {
        KEEP(*(.requests))
    } :load

    .requests.end : ALIGN(8)
    {
        KEEP(*(.requests_end))
    } :load

    .bss (NOLOAD) : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } :load

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.comment)
    }
}
