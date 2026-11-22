---
id: n7j9
title: "M8: Finalize the BMS hardware, master and slave"
type: epic
status: open
priority: P1
created: "2026-11-22T10:52:13.873907220Z"
updated: "2026-11-22T10:52:13.873907220Z"
tags:
  - bms
  - hw
  - safety
depends_on:
  - 6duk
---

## Goal
BMS master boards for the front and rear packs and a cell-monitoring slave board for each Renault Zoe module, designed, built and brought up.

## Scope
- The master in `ecus/bms/hardware/master`: clean ERC and DRC, current shunt and isolated ADC, contactor coil drivers, CAN2.
- The slave in `ecus/bms/hardware/slave`, which is empty today: the analog front end, isoSPI daisy chain and mounting on the module.
- Ordering, assembly and bring-up. Tags `bms-hw-revA` per board.

Mostly manual. The slave works on the HV pack, so its isolation and creepage get their own review.

## Decisions
- Shunt BAS-M-R0001-R-5.0 with an AMC3306M05 isolated ADC, as on the `bms` page.
- The master moves from the STM32G491CC to an STM32G474 with 512 KB, the same part as the VCM main MCU.

## Safety
Isolation between the pack and the LV side holds on every board. The master can open its contactor whatever CAN2 says.

## Acceptance
- [ ] `just bms ci-hardware` passes for both boards.
- [ ] Bring-up checklist completed for both.

## Verified on hardware
- Cell voltages and temperatures from one module read correctly over isoSPI against a meter; shunt current against a reference; isolation tested before connecting a pack.