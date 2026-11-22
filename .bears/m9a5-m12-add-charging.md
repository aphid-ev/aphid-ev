---
id: m9a5
title: "M12: Add charging"
type: epic
status: open
priority: P2
created: "2026-11-22T10:52:44.792643891Z"
updated: "2026-11-22T10:52:44.792643891Z"
tags:
  - vcm
  - bms
  - crates
  - safety
depends_on:
  - qg2z
---

## Goal
The car charges from AC through the Mitsubishi OBC, with the BMS limiting current to what the cells accept, and runs its 12 V DC/DC whenever it is on.

## Scope
- The OBC and DC/DC messages on CAN1 in `crates/ev-can`.
- Charge control in the VCM: plug and pilot detection, the request to the OBC, stopping on BMS limits or faults.
- Charge-current limits from the BMS over CAN2.
- Charge status in the manager.
- The `obc` and `ev-can` wiki pages.

## Decisions
None yet. The charge inlet and pilot handling still have to be decided.

## Safety
Charging stops and the contactors open on any BMS limit, lost CAN peer or interlock fault, as specified in M1.

## Acceptance
- [ ] Charge-control logic covered by host tests, including every stop condition.

## Verified on hardware
On the bench, then in the car: charge at a limited current, see it taper at the cell limit, and see it stop when the plug is pulled or a fault is injected.