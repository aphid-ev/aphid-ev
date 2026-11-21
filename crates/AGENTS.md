# Shared crate guidelines

These crates are compiled into firmware, the bootloader and desktop code.

- `#![no_std]`, no allocator. `std` only behind an off-by-default `std`
  feature or in tests.
- No dependency on a HAL, `cortex-m` or anything MCU-specific. Hardware is
  reached through traits (`embedded-hal`, `embedded-can`) or passed in by the
  firmware, so every crate here builds and tests on the host.
- The bootloader links some of these crates and has a tight size budget. Keep
  dependencies minimal and gate anything it does not need behind a feature.
- A crate here is the single definition of what it describes. CAN IDs, message
  layouts and parameters are defined once and imported everywhere; never
  duplicate a constant in a firmware or in the app.
- Wire formats are compatibility surfaces between ECUs that may run different
  firmware versions. Changing one is a breaking change: update every user in
  the same commit and say so in the commit message (`feat(protocol)!: …`).
- Behaviour belongs here with tests, not in firmware binaries. Time-dependent
  logic takes time as an input so tests can drive it.
- Verify with `just host ci`, and build at least one firmware to confirm the
  crate still compiles without `std`.
