# Bootloader

CAN bootloader shared by the ECUs. Plain `no_std` Rust without embassy to keep
it small: the STM32G4 PAC and the `fdcan` crate, in the first 8 KiB of flash.
One source tree builds for each chip through a cargo feature; the chips are
listed in the `justfile`.

| Feature | Chip | Flash |
| --- | --- | --- |
| `stm32g431cx` | STM32G431 | 128 KiB in 2 KiB pages |
| `stm32g491rx` | STM32G491 | 512 KiB in 4 KiB pages |

`can-echo/` is a minimal application for testing a flash over CAN. It links
at the application offset, `0x08002000`, and echoes every frame it receives,
so a successful flash shows up on the bus. It is its own cargo workspace.

```sh
just bootloader build <feature> <target>   # one chip
just bootloader build-echo                 # the test application
just bootloader ci                         # fmt, lint and build all of it
```

The update protocol is defined in
[`crates/boot-protocol`](../../crates/boot-protocol/) and specified on
[the wiki](https://bladlus.se/wiki/bootloader/).
