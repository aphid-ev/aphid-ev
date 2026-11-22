---
id: zeyx
title: "M7: Add VCM configuration and diagnostics to the manager"
type: epic
status: open
priority: P2
created: "2026-11-22T10:52:13.855986953Z"
updated: "2026-11-22T10:52:13.855986953Z"
tags:
  - manager
  - vcm
depends_on:
  - mtyu
  - 6y59
---

## Goal
From the manager, the user can read and change the VCM's parameters (pedal calibration, torque maps, limits), watch live data and read fault codes.

## Scope
- A VCM page in the manager, using the parameter protocol from M1.
- Live data from CAN1 and CAN2, fault codes and a bus log you can save.
- Pedal calibration as a guided procedure.

## Decisions
None yet.

## Safety
Parameters that bound torque or limits are validated in the VCM against fixed bounds, not only in the app. Changing them while driving is refused.

## Acceptance
- [ ] Core logic tested against the fake bus in `just host ci`.

## Verified on hardware
- Calibrate the pedal on the bench and see the new range take effect after a restart.