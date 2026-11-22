---
id: vtzz
title: "M10: Add BMS configuration and monitoring to the manager"
type: epic
status: open
priority: P2
created: "2026-11-22T10:52:34.415458585Z"
updated: "2026-11-22T10:52:34.415458585Z"
tags:
  - manager
  - bms
depends_on:
  - mtyu
  - 2h7v
---

## Goal
From the manager, the user can see every cell voltage and temperature, state of charge and current, read BMS faults and change BMS parameters.

## Scope
- A BMS page in the manager: per-cell view for both packs, balancing state, history log.
- BMS parameters through the protocol from M1.

## Decisions
None yet.

## Safety
Cell and current limits are bounded in the BMS firmware. The app cannot set them outside those bounds.

## Acceptance
- [ ] Core logic tested against the fake bus in `just host ci`.

## Verified on hardware
- Cell voltages shown in the app agree with a meter on one module.