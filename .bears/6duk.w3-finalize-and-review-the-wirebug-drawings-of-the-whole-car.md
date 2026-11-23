---
id: 6duk.w3
title: Finalize and review the wirebug drawings of the whole car
status: in_progress
priority: P0
created: "2026-11-23T06:28:12.569410Z"
updated: "2026-11-23T06:48:49.168487Z"
tags:
  - wiring
  - site
attempts: 1
---

## Summary
Go through the wirebug model of the whole vehicle, drawing by drawing, and bring it to a state the user has reviewed and accepted. The rest of the system design (ECU pinouts, contactor specification, board design) builds on these drawings.

## Component
`wiring/`, and the wiki pages in `site/` that embed its drawings.

## Acceptance criteria
- [ ] Every electrical subsystem of the car is in the model: high-voltage pack and contactors, 12 V supply and fusing, CAN buses, the ECUs, charging, and the retained original Beetle circuits.
- [ ] Every drawing has been rendered and reviewed by the user, and the review comments are resolved.
- [ ] The drawings agree with the wiki pages that describe the same circuits; every disagreement is reported and settled.
- [ ] `just wiring check` passes and `just docs` builds.

## Implementation notes
- The user decides the design. Ask before adding or changing a circuit the wiki or model does not already fix.
- If wirebug lacks a capability a drawing needs, say so instead of working around it.
- Detailed ECU connector pin assignment is `6duk.55`; this task covers the vehicle-level drawings.

## Verification
`just wiring check`, `just wiring render`, `just docs`; the user signs off on the rendered drawings.