ENTRY(_start)

SECTIONS
{
    . = 0x400000 + SIZEOF_HEADERS;

    .text : {
        *(.text._start)
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    /DISCARD/ : {
        *(.comment)
        *(.note.*)
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.gnu.hash)
        *(.dynsym)
        *(.dynstr)
        *(.dynamic)
        *(.got)
        *(.got.plt)
        *(.plt)
        *(.rela.*)
        *(.debug_*)
    }
}
