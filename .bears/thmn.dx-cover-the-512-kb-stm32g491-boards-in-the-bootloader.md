---
id: thmn.dx
title: Cover the 512 KB STM32G491 boards in the bootloader
status: open
priority: P2
created: "2026-11-22T10:20:58.818275492Z"
updated: "2026-11-22T22:52:31.474931751Z"
tags:
  - bootloader
  - vcm
  - bms
depends_on:
  - 6duk.th
---

## Summary
Every DIY ECU uses the STM32G4 (`6duk.th`). The bootloader builds for the STM32G431, which covers the dongle and the VCM monitor, and for the 512 KB STM32G491 (`stm32g491rx`), which the VCM main MCU (an xE part, package not chosen yet) and both BMS masters (STM32G491CE) use. The xE parts share one flash layout whatever the package, so this is mostly naming and checking, not a new port.

## Component
`ecus/bootloader`, and `ecus/bootloader/AGENTS.md` and `README.md`.

## Acceptance criteria
- [ ] The G491 feature's name and docs say it covers every 512 KB G491 package (CE and RE), for example by renaming `stm32g491rx` to `stm32g491xe`, with the justfile's `families` updated to match.
- [ ] The `FlashLayout` for it is checked against RM0440: 512 KB, single bank, 128 pages of 4 KiB, and the page count fits the 7-bit `PNB` field.
- [ ] `just bootloader ci` builds every family and the size stays under 8 KiB.

## Edge cases
- Erasing must never reach the bootloader's own pages.

## Verification
`just bootloader ci`. Bench check on a G491 board is part of M3's hardware verification.