---
id: 6duk.th
title: Choose the MCU for every board
status: done
priority: P0
created: "2026-11-22T10:53:14.236750818Z"
updated: "2026-11-22T11:06:49.460246205Z"
tags:
  - manual
  - vcm
  - bms
  - can-usb
---

## Summary
Decide the MCU for the dongle, the VCM main and monitor, and the BMS master. Each family is a bootloader port, an embassy feature and a probe-rs target to maintain. Today there are four: STM32G431 (dongle), STM32F405RG and STM32F091CC (VCM), STM32G491CC (BMS master).

## Steps
- Decide whether the VCM moves to G4 parts (FDCAN, page flash, one bootloader family), and whether the monitor keeps a different family on purpose for diversity.
- Record the choice, with the reason, on the `vcm` and `bms` wiki pages.

## Expected result
A list of parts, which `thmn.dx` (bootloader families) and the VCM and BMS hardware epics build on.

## Decision (2026-11-22)
Every DIY ECU uses the STM32G4 family:

| Board | MCU |
| --- | --- |
| VCM main | STM32G474, 512 KB dual-bank flash |
| VCM monitor | STM32G431, a separate chip with its own supply and clock |
| BMS master (front and rear) | STM32G474 |
| CAN/USB dongle | STM32G431CB, unchanged |

Considered and not taken:
- **STM32H7:** 128 KB flash sectors, a more complex supply, and far more speed than a VCM needs.
- **Dual-core H7 as the monitor:** the two cores share supply, clock, reset and CAN, which are common-cause failures for a supervisor.
- **STM32H5:** the fallback if 512 KB ever runs short.
- **STM32G5:** embassy-stm32 0.6.0 has no support for it.

Packages are chosen with each board's redesign. The wiki's `vcm` and `bms` pages record the choice.
