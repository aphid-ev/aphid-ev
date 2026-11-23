---
id: thmn.am
title: Validate the application image before the bootloader jumps to it
status: done
priority: P1
created: "2026-11-22T09:23:05.885427495Z"
updated: "2026-11-23T19:59:41.712201990Z"
tags:
  - bootloader
  - crates
  - safety
attempts: 1
---

The bootloader starts whatever is in the application region after the wakeup window, including erased flash or a half-written image, which `ecus/bootloader/AGENTS.md` forbids. Design an image header or trailer (length, CRC, version) checked by logic in `crates/boot-protocol` with host tests, and stay in the bootloader when it fails. This changes the flash layout and every application's `memory.x`, and it gives the wakeup frame a real version in place of the hard-coded 1.0.0. Update the wiki page `bootloader`.

## Decisions
- A 16-byte header at offset `0x200`, right after the vector table: magic, length, CRC-32/ISO-HDLC and version. The bootloader computes the CRC with the STM32G4 CRC unit. `tools/boot-image` seals it after linking.
- No bootloader is in the field yet, so the protocol may change: the Wakeup frame gains an image status byte, and error 7 (InvalidImage) refuses Start application.
- Only `can-echo` moves to the application offset. The ECU firmware keeps booting from `0x08000000` with a debug probe until it is closer to done (yng2).