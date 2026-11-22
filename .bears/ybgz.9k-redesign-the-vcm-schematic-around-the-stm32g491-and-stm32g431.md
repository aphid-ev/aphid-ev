---
id: ybgz.9k
title: Redesign the VCM schematic around the STM32G491 and STM32G431
status: open
priority: P1
created: "2026-11-22T11:07:08.864077277Z"
updated: "2026-11-22T22:52:31.520218247Z"
tags:
  - manual
  - vcm
  - hw
  - safety
depends_on:
  - 6duk.th
---

## Steps
- Replace the STM32F405RG with a 512 KB STM32G491 (xE) and the STM32F091CC with an STM32G431 in `ecus/vcm/hardware`, choosing packages for the pinout from `6duk.55`.
- Give the monitor its own regulator and clock source and its own CAN transceiver on each bus it watches, and let it drive the contactor high side (and the inverter enable, if wired) independently of the main MCU.
- Update the `vcm` wiki page's microcontroller section, which notes that the schematic still has the old parts.

## Expected result
`just vcm erc` passes, and the parts and pins can be read off the schematic for the firmware.