# Bootloader

CAN bootloader shared by the ECUs. Plain `no_std` Rust without embassy to keep
it small. One source tree builds for each STM32 family through a cargo
feature; the families are listed in the `justfile`.

```sh
just bootloader build <feature> <target>   # one family
just bootloader ci                         # fmt, lint and build every family
```

The update protocol is defined in [`crates/`](../../crates/) and shared with
the application firmware and the manager app.
