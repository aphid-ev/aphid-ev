# boot-image

Seals a linked application ELF for the bootloader: fills in the length and
CRC of its image header, writes the sealed header back into the ELF, and
writes the raw image for flashing over CAN.

```sh
cargo run -p boot-image -- <elf> [-o <bin>]
```

The header format is specified on
[the wiki](https://bladlus.se/wiki/bootloader/#image-header).
