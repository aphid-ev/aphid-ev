---
id: 6duk.th
title: Choose the MCU for every board
status: done
priority: P0
created: "2026-11-22T10:53:14.236750818Z"
updated: "2026-11-22T22:52:31.460316821Z"
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

## Decision (2026-11-22, revised the same day)
Every DIY ECU uses the STM32G4 family:

| Board | MCU |
| --- | --- |
| VCM main | STM32G491 with 512 KB flash (xE); package chosen with the redesign |
| VCM monitor | STM32G431, a separate chip with its own supply and clock |
| BMS master (front and rear) | STM32G491CE, 512 KB, the LQFP48 drop-in for the current G491CC |
| CAN/USB dongle | STM32G431CB, unchanged |

The VCM main and the BMS masters share one 512 KB part, so the bootloader needs one layout for both. It already has it (`stm32g491rx`, 128 pages of 4 KiB).

Considered and not taken:
- **STM32G474:** first choice, for its dual-bank flash, which lets a new image be written to the idle bank and swapped in once checked. Firmware is flashed over CAN in the garage and the bootloader stays in control when an update fails, so bank swap is not needed. The G474's other extras (HRTIM, more ADCs, DACs and analog) are for power conversion, which no board does. It would also need a new bootloader port with bank-select erase.
- **STM32H7:** 128 KB flash sectors, a more complex supply, and far more speed than a VCM needs.
- **Dual-core H7 as the monitor:** the two cores share supply, clock, reset and CAN, which are common-cause failures for a supervisor.
- **STM32H5:** the fallback if 512 KB ever runs short.
- **STM32G5:** embassy-stm32 0.6.0 has no support for it.

The wiki's `vcm` and `bms` pages record the choice.