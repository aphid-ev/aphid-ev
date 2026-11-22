# Bootloader

CAN bootloader shared by the ECUs. Plain `no_std` Rust without embassy to keep
it small: the STM32G4 PAC and the `fdcan` crate, in the first 8 KiB of flash.
One source tree builds for each chip through a cargo feature; the chips are
listed in the `justfile`.

| Feature | Chip | Flash |
| --- | --- | --- |
| `stm32g431cx` | STM32G431 | 128 KiB in 2 KiB pages |
| `stm32g491rx` | STM32G491 | 512 KiB in 4 KiB pages |

```sh
just bootloader build <feature> <target>   # one chip
just bootloader ci                         # fmt, lint and build every chip
```

The update protocol is defined in
[`crates/boot-protocol`](../../crates/boot-protocol/) and specified on
[the wiki](https://bladlus.se/wiki/bootloader/).
