---
id: thmn.dx
title: Cover the 512 KB STM32G491 boards in the bootloader
status: done
priority: P2
created: "2026-11-22T10:20:58.818275492Z"
updated: "2026-11-23T20:16:12.567394650Z"
tags:
  - bootloader
  - vcm
  - bms
depends_on:
  - 6duk.th
attempts: 1
---

## Summary
Every DIY ECU uses the STM32G4 (`6duk.th`). The bootloader builds for the STM32G431, which covers the dongle and the VCM monitor, and for the 512 KB STM32G491 (`stm32g491rx`), which the VCM main MCU (an xE part, package not chosen yet) and both BMS masters (STM32G491CE) use. The xE parts share one flash layout whatever the package, so this is mostly naming and checking, not a new port.

## Component
`ecus/bootloader`, and `ecus/bootloader/AGENTS.md` and `README.md`.

## Acceptance criteria
- [x] The G491 feature's name and docs say it covers every 512 KB G491 package (CE and RE), for example by renaming `stm32g491rx` to `stm32g491xe`, with the justfile's `families` updated to match.
- [x] The `FlashLayout` for it is checked against RM0440: 512 KB, single bank, and the page count fits the `PNB` field.
  The check found that the expected "128 pages of 4 KiB" was wrong. The G491 is a category 4 part with 256 pages of 2 KiB and an eight-bit `PNB`, according to an ST moderator citing RM0440 table 18 and to ST's chip data in `stm32-metapac` 21.0.0 (`flash_g4c4`). The `stm32g4` PAC's seven-bit `PNB` for the G491 is an SVD error. The old layout would have erased the bootloader's upper 4 KiB on an Erase.
- [x] `just bootloader ci` builds every family and the size stays under 8 KiB.

## Edge cases
- Erasing must never reach the bootloader's own pages.

## Verification
`just bootloader ci`. Bench check on a G491 board is part of M3's hardware verification.