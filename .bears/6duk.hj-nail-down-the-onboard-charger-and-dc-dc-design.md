---
id: 6duk.hj
title: Nail down the onboard charger and DC-DC design
status: done
priority: P1
created: "2026-11-23T08:21:10.771454Z"
updated: "2026-11-23T18:35:32.228675Z"
tags:
  - site
  - wiring
  - vcm
attempts: 1
---

## Summary
The Mitsubishi Outlander OBC/DC-DC replaced the Leaf PDM because the PDM does not fit the car. What the unit needs from the VCM and from the 12 V and HV systems has not been worked through since. Settle it so the VCM I/O and the harness can be finalized.

## Component
The `obc` wiki page, `wiring/` (charge inlet, OBC/DC-DC, VCM ports), and the VCM I/O list on the `vcm` page.

## To decide
- The charge inlet part, its lock actuator, any lock-position feedback and the status LEDs, and the VCM outputs and inputs they need.
- How CP and PP are handled: CP goes to the OBC and PP to the VCM today. What wakes the VCM when a cable is plugged in while the car is off.
- The control interface: which CAN1 messages the unit needs and sends, and the roles of DC SW, IGCT and SENSE.
- The charging sequence against the VCM operating states, including when the contactors close and when the DC-DC runs.
- The 12 V side: the DC-DC output current, the cable size and the MIDI fuse rating in the primary box.
- The HV side: the fused branch from the junction box, and that the unit has no interlock.
- Cooling: whether the unit is on the coolant loop and what that asks of the pump while charging.

## Acceptance criteria
- [ ] The `obc` page states every decision above, with sources for facts about the unit.
- [ ] Every signal between the inlet, the OBC/DC-DC and the VCM is in the wiring model, and the "VCM I/O" drawing shows it.
- [ ] `just wiring check` passes and the site builds.

## Implementation notes
- The user decides the design. Ask before fixing a part or a signal the wiki does not already fix.
- CAN message definitions belong in the `ev-can` crate; this task only lists which messages are needed.

## Verification
`just wiring check`, `just docs`.