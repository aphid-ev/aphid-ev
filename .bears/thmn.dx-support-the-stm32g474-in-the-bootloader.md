---
id: thmn.dx
title: Support the STM32G474 in the bootloader
status: open
priority: P2
created: "2026-11-22T10:20:58.818275492Z"
updated: "2026-11-22T11:07:08.836067211Z"
tags:
  - bootloader
  - vcm
  - bms
depends_on:
  - 6duk.th
---

## Summary
Every DIY ECU now uses the STM32G4 (`6duk.th`). The bootloader already builds for the STM32G431, which covers the VCM monitor. Add the STM32G474, used by the VCM main MCU and both BMS masters.

## Component
`ecus/bootloader`, and `ecus/bootloader/AGENTS.md` and `README.md`.

## Acceptance criteria
- [ ] An `stm32g474` feature with a `FlashLayout` for 512 KB, listed in the justfile's `families`.
- [ ] `just bootloader ci` builds every family and the size stays under 8 KiB.

## Implementation notes
- Dual-bank mode (the default for the G474's 512 KB) uses 2 KB pages, 128 per bank. Erasing a page needs the `BKER` bank-select bit as well as `PNB`, which the G431 code does not set. Decide whether `FlashLayout` models banks or the application region stays in one bank.
- Bank swap for A/B updates is a separate decision, belonging to the image-validation task `thmn.am`. Keep the layout open to it.
- The `stm32g491rx` feature (512 KB, 4 KB pages) no longer has a board. Remove it, or keep it only if the user wants it.

## Edge cases
- Erasing must never reach the bootloader's own pages in bank 1.

## Verification
`just bootloader ci`. Bench check on a G474 board is part of M3's hardware verification.