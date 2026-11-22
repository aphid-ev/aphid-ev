---
id: ybgz
title: "M5: Finalize the VCM hardware"
type: epic
status: open
priority: P1
created: "2026-11-22T10:51:52.612616651Z"
updated: "2026-11-22T10:52:02.954188149Z"
tags:
  - vcm
  - hw
  - safety
depends_on:
  - 6duk
---

## Goal
A VCM board that is designed, built and brought up, tagged `vcm-hw-revA`.

## Scope
- Schematic and layout in `ecus/vcm/hardware`, to the pinout and MCUs fixed in M1.
- Clean ERC and DRC under `just vcm ci-hardware`.
- Ordering, assembly and bring-up: supplies, both MCUs on the probe, CAN1 and CAN2 transceivers, inputs and high-side outputs.
- The `vcm` wiki page's hardware section kept true.

Mostly manual: KiCad work, ordering and bench checks are the user's.

## Decisions
- Enclosure: Cinch ModICE ME-MX 24 with Molex MX-150, as on the `vcm` page.
- Main MCU STM32G474, monitor STM32G431 as a separate chip with its own regulator and clock, replacing the STM32F405RG and STM32F091CC in the current schematic.

## Safety
The monitor MCU must be able to cut the contactor high-side supply on its own, whatever the main MCU does.

## Acceptance
- [ ] `just vcm ci-hardware` passes.
- [ ] Bring-up checklist completed on revision A.

## Verified on hardware
- Supply rails, both MCUs reachable with probe-rs, both CAN buses passing frames with the dongle, and the monitor cutting the high-side output.