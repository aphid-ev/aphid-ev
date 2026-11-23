/* This is the application image the bootloader flashes, so it links at the
   application offset rather than at the start of flash: the first 8K belong
   to the bootloader. Keep in sync with BOOTLOADER_SIZE in
   ../src/main.rs and with the FLASH region in ../memory.x.

   Linked at 0x08000000 instead, the image would still be written to the
   application region, but its vector table would point into the bootloader
   and the jump would land in the wrong code. */
MEMORY
{
    FLASH : ORIGIN = 0x08002000, LENGTH = 120K
    RAM : ORIGIN = 0x20000000, LENGTH = 32K
}

/* The image header, at a fixed 0x200 from the start of the image
   (boot_protocol::image::HEADER_OFFSET), right after the vector table. The
   bootloader looks for it there and the image tool seals it there.
   `boot_protocol::image_header!()` in the application fills the section.

   The section starts where the vector table ends and fills the space up to
   0x200 with 0xff, the value of erased flash. Left as a gap, the linker
   zero-fills it in the ELF segment while objcopy and flash tools that work
   by section leave it erased, and the CRC would depend on the tool that
   flashed the image. */
SECTIONS
{
    .image_header :
    {
        FILL(0xff);
        . = ALIGN(0x200);
        __image_header = .;
        KEEP(*(.image_header));
        __eimage_header = .;
    } > FLASH
} INSERT AFTER .vector_table;

/* cortex-m-rt starts .text right after the vector table unless told
   otherwise. Start it after the header instead. */
_stext = ORIGIN(FLASH) + 0x210;

ASSERT(__image_header == ORIGIN(FLASH) + 0x200,
    "The vector table runs into the image header at 0x200.");
ASSERT(__eimage_header - __image_header == 16,
    "The image header is missing: call boot_protocol::image_header!() in the application.");
