# Shared crates

`no_std` Rust crates shared between firmware, the bootloader and host code:
the CAN protocol, the parameter model and the bootloader protocol. They are
members of the host workspace, so `just host test` runs their tests on the
desktop.
