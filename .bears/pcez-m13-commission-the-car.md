---
id: pcez
title: "M13: Commission the car"
type: epic
status: open
priority: P1
created: "2026-11-22T10:52:44.810326070Z"
updated: "2026-11-22T10:52:49.647043695Z"
tags:
  - vcm
  - bms
  - safety
depends_on:
  - qg2z
  - zeyx
  - vtzz
  - m9a5
---

## Goal
The Aphid drives, regenerates and charges on the road.

## Scope
- Installing the harness and ECUs in the car.
- First power-up on LV, then HV with the car on stands.
- First drive at reduced torque limits, raising them step by step.
- First charge in the car once M12 is done.
- Releases: firmware tags for every unit and the manager, cut by the user.

Entirely manual.

## Decisions
None yet.

## Safety
Each step has a written checklist and starts from limits reduced through the manager. The car is first driven off public roads.

## Acceptance
- [ ] Checklists completed for power-up, first drive and first charge.

## Verified on hardware
This epic is the verification.