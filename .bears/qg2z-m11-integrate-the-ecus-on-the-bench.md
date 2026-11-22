---
id: qg2z
title: "M11: Integrate the ECUs on the bench"
type: epic
status: open
priority: P1
created: "2026-11-22T10:52:34.432223031Z"
updated: "2026-11-22T10:52:34.432223031Z"
tags:
  - vcm
  - bms
  - safety
depends_on:
  - ybgz
  - 6y59
  - n7j9
  - 2h7v
---

## Goal
The VCM, both BMS controllers, the inverter and the dongle run together on a bench, first at low voltage and then on HV, with the contactor sequence and every fault reaction exercised before anything goes in the car.

## Scope
- A bench harness built from the wiring model.
- Low voltage: CAN1 and CAN2 traffic, wake-up and shutdown, firmware updates of every ECU from the manager.
- HV with current limited: precharge, contactor close and open, the inverter spinning the motor unloaded.
- Fault injection: each fault reaction from M1, one at a time.
- Fixes go back into the epic that owns the code.

Almost entirely manual.

## Decisions
None yet.

## Safety
HV work follows a written procedure: isolation checked first, current-limited supply, a second person present. Every fault reaction in M1 is exercised and recorded.

## Acceptance
- [ ] A recorded result for each fault reaction from M1.

## Verified on hardware
This epic is the verification.