---
id: yng2
title: Link the ECU firmware at the application offset and seal its images
status: open
priority: P3
created: "2026-11-23T19:59:32.483566839Z"
updated: "2026-11-23T19:59:32.483566839Z"
tags:
  - vcm
  - bms
  - can-usb
  - bootloader
---

Only `ecus/bootloader/can-echo` runs under the bootloader. The VCM main and monitor firmware and the can-usb firmware link at `0x08000000` through embassy's `memory-x` feature, and are flashed with a debug probe. That is the plan until they are closer to done (decided while working on thmn.am).

When a firmware moves over:
- Drop `memory-x` and add a `memory.x` like `can-echo`'s: `FLASH` at `0x08002000`, the `.image_header` section at `0x200`, and `_stext` after it.
- Call `boot_protocol::image_header!()`.
- Seal the ELF with `tools/boot-image` in the firmware's `build` recipe, and flash the sealed ELF in `flash`.
- Remove the matching bullet from the limitations on the `bootloader` wiki page.

This is a flash layout change for that firmware: a `!` commit that says the bootloader must be flashed first.