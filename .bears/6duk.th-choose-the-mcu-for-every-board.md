---
id: 6duk.th
title: Choose the MCU for every board
status: open
priority: P0
created: "2026-11-22T10:53:14.236750818Z"
updated: "2026-11-22T10:53:14.236750818Z"
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