---
id: 6duk.55
title: Model every ECU's connectors and pins in the wiring model
status: open
priority: P1
created: "2026-11-22T10:53:27.308439893Z"
updated: "2026-11-22T10:53:27.308439893Z"
tags:
  - wiring
---

## Summary
Finish the harness in `wiring/` so every ECU's connector has its pins assigned, including the direction shifter on the VCM. The boards are then designed to this pinout.

## Component
`wiring/`, and the wiki pages that embed its drawings.

## Acceptance criteria
- [ ] Every signal into the VCM, both BMS controllers and the dongle is on a named connector pin.
- [ ] `just wiring check` passes and `just docs` builds.

## Implementation notes
- The user decides the pinout; ask before assigning a pin the schematic or wiki does not already fix.
- The pin assignment in each schematic is the reference. Report every place where the model and a schematic disagree.

## Verification
`just wiring check`, `just docs`.