# Bootloader guidelines

In addition to `ecus/AGENTS.md`.

- Plain `no_std` Rust. No embassy, no async executor, no allocator, no
  formatting machinery (`core::fmt`, `defmt` in release builds). Size is the
  constraint: check the binary size before and after a change and report it.
- One source tree, one cargo feature per MCU family. Family-specific code sits
  behind that feature in its own module; everything else is shared. A change
  must build for every family: `just bootloader ci`.
- The flash layout (bootloader, application, any metadata) is a contract with
  every application firmware's `memory.x` and with the image tool. Changing
  it means changing all of them in the same commit.
- The update protocol comes from the shared crate in `crates/`. Bootloaders in
  the field cannot be updated over CAN, so treat the protocol as frozen:
  extend it compatibly, never change existing behaviour.
- A bug here can leave an ECU unrecoverable without a debug probe. Validate
  every image before jumping to it, never erase the bootloader's own region,
  and keep the path from reset to "stay in bootloader" as short as possible.
- Protocol and image-validation logic belongs in the shared crate with host
  tests. This crate holds only the flash, CAN and jump code for each family.
