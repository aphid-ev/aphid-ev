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

## Things to know

- Exactly one chip feature must be selected. With none or both, `stm32g4`'s
  own build script fails first with an unhelpful message, so check the
  feature flag first when a build breaks oddly.
- Adding a chip is a feature, a `FlashLayout` and a `PNB_BITS` in
  `src/flash.rs`. `FlashLayout::new` asserts at compile time that the page
  count fits `PNB`. Take the page size and the `PNB` width from RM0440 or
  `stm32-metapac`, not from the `stm32g4` PAC: its `PNB` is seven bits on
  every G4, but the G491 (category 4) has eight bits and 256 pages of
  2 KiB. A wrong page size makes an erase hit the bootloader's own pages.
- The flash layout is `BOOTLOADER_SIZE` in `src/main.rs`, the `FLASH` region
  in `memory.x` and the `ORIGIN` in each application's `memory.x`
  (`can-echo/memory.x` among them). An application linked at `0x08000000`
  still gets written to the application region, but its vector table points
  into the bootloader.
- The image header is part of that contract: `HEADER_OFFSET` in
  `crates/boot-protocol/src/image.rs`, the `.image_header` section and
  `_stext` in each application's `memory.x`, and the image tool
  `tools/boot-image`. An image that was not sealed by the tool fails
  validation, and the node stays in the bootloader.
- Image validation runs on every reset, before the wakeup frame. Code on
  that path must not panic: `panic-halt` would stop the node before it can
  be reached over CAN, on every reset.
- The clock setup duplicates the ECU firmware's: 16 MHz HSE, 144 MHz
  sysclk, 48 MHz FDCAN. The bootloader sets five flash wait states where
  embassy computes four, which is safe but not identical.
- The jump to the application does not de-initialise the clocks, so the
  application's `embassy_stm32::init` starts from a configured RCC. Every
  peripheral the bootloader uses is reset in `state_application`; one you
  add goes there too.
