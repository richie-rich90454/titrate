/* Freestanding x86-64 multiboot2 image.
 * Link with -nostdlib -nostartfiles -nodefaultlibs -T x86_64-none.ld and
 * -C panic=abort. Any hosted object pulled in here will fail to satisfy these
 * sections rather than silently growing the image. */
ENTRY(_start)
OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)

SECTIONS
{
    /* Multiboot2 requires the image header in the first megabyte. */
    . = 1M;

    .boot : ALIGN(8)
    {
        KEEP(*(.multiboot))
        *(.boot)
    }
    ASSERT(SIZEOF(.boot) >= 24, "missing multiboot2 header")

    .text : ALIGN(4096)
    {
        *(.text .text.*)
    }

    .rodata : ALIGN(8)
    {
        *(.rodata .rodata.*)
    }

    . = ALIGN(4096);
    .data :
    {
        *(.data .data.*)
    }

    .bss (NOLOAD) :
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    . = ALIGN(4096);
    _end = .;

    /* No CRT, exception tables, or debug data may enter the image. */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.GNU-stack)
        *(.debug*)
    }
}
