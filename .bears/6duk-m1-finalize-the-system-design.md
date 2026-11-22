---
id: 6duk
title: "M1: Finalize the system design"
type: epic
status: open
priority: P0
created: "2026-11-22T10:51:28.180205011Z"
updated: "2026-11-22T10:51:28.180205011Z"
tags:
  - wiring
  - crates
  - site
  - safety
---

## Goal
Everything the ECUs and the app are built against is decided and written down: what each ECU connects to, what it says on CAN2, what can be configured, and how the HV contactors are sequenced and shut down.

## Scope
- The harness in `wiring/`: every signal, connector and pin per ECU, so the VCM and BMS boards have a fixed pinout to design to.
- The CAN2 message set between the VCM and the BMS controllers, as a shared crate with host tests.
- The parameter model and the protocol for reading and writing parameters over CAN, as a shared crate.
- Node IDs for every ECU on CAN2, which the bootloader needs.
- The contactor and precharge sequence across the VCM monitor and both BMS controllers, and the fault reactions, on the `contactors` and `safety` wiki pages.
- The MCU for each board, and whether the monitor uses a different family on purpose.

Out of scope: board layout and firmware behaviour.

## Decisions
- The direction shifter is wired to the VCM. There is no direction controller ECU.
- CAN1 is the OEM powertrain bus (EV-CAN), CAN2 the DIY bus, as on the `ev-can` page.

## Open questions
- MCU per board. Four families today (G431, F405, F091, G491CC) mean four bootloader ports. Moving the VCM to G4 parts would leave one.
- CAN2 bit rate.

## Safety
The contactor sequence, precharge timing and limits, and the reaction to every fault (lost CAN2 peer, interlock open, monitor disagreement, cell limit) are written by the user. Implementers follow them; they do not choose them.

## Acceptance
- [ ] `just wiring check` passes with every ECU's connectors modelled.
- [ ] CAN2 messages and parameters are defined in `crates/`, with host tests.
- [ ] The contactor sequence and fault reactions are on the wiki.
- [ ] Each board's MCU is recorded on its wiki page.

## Verified on hardware
Nothing. This is design work.