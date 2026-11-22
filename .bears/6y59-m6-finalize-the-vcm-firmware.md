---
id: 6y59
title: "M6: Finalize the VCM firmware"
type: epic
status: open
priority: P1
created: "2026-11-22T10:52:02.970861196Z"
updated: "2026-11-22T10:52:02.970861196Z"
tags:
  - vcm
  - crates
  - safety
depends_on:
  - 6duk
  - thmn
---

## Goal
Main and monitor firmware that drive the car: torque from the pedal, direction, contactor authorization, cooling and lights, with the monitor supervising the main MCU.

## Scope
- Behaviour in `crates/vcm-core` and `crates/ev-can` with host tests; `ecus/vcm/firmware/{main,monitor}` wire it to the peripherals.
- Main: throttle, direction, torque requests on EV-CAN, inverter status, CAN2 messages and parameters from M1, high-side outputs.
- Monitor: throttle plausibility, torque cross-check, main/monitor heartbeat, interlock and the contactor supply.
- Linked behind the bootloader from M3.

Charging is M12. Configuration pages are M7.

## Decisions
- Starts on Nucleo boards before revision A exists, since the logic is host-tested. Pin mapping follows the schematic once M5 fixes it; a firmware states which board revisions it supports.

## Safety
Torque, direction, contactor authorization and every fault reaction follow the behaviour written in M1. Each is covered by a host test.

## Acceptance
- [ ] `just vcm ci-firmware` and `just host ci` pass.
- [ ] No panicking paths while driving.

## Verified on hardware
On revision A with the inverter on the bench at low voltage: pedal to torque request, direction changes only as specified, and the monitor opening the contactor supply on each injected fault.