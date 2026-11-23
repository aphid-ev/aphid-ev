---
id: 6duk.55
title: Model every ECU's connectors and pins in the wiring model
status: open
priority: P1
created: "2026-11-22T10:53:27.308439893Z"
updated: "2026-11-23T18:12:48.433173Z"
tags:
  - wiring
depends_on:
  - 6duk.w3
  - 6duk.hj
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
- The VCM's interlock input, safe power output and EV relay output have no connector in the model. Until they do, the leads to them are missing from the harness drawings.
- The VCM's connectors are placeholders, one per function. The wiring review swapped pins 3 and 4 of the placeholder accelerator connector to straighten a drawing.
- The VCM has one always-on feed behind a 15 A fuse that also supplies its high-side outputs. Check the terminal rating of its B+ and ground pins against that, or gang pins.
- Each BMS controller's contactor outputs are not on a connector either, and its two HV sense pairs and the front BMS's shunt pair are now wired inside the packs.

## Verification
`just wiring check`, `just docs`.