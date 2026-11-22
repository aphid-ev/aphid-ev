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
