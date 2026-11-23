---
id: 6duk.yq
title: Choose the HV connectors, the junction box fuse and the MSD
status: open
priority: P2
created: "2026-11-23T18:12:37.827742Z"
updated: "2026-11-23T18:12:37.827742Z"
tags:
  - wiring
  - site
---

## Summary
Every HV connector in the wiring model is a generic two-pin placeholder: the two pack connectors, the three on the HV junction box and the inverter's DC input. The 30 A fuse for the OBC/DC-DC branch has no type, and the MSD is a candidate, not a chosen part.

## To decide
- The connector or gland and lug for each HV cable end, with part numbers.
- The fuse type for the 30 A OBC/DC-DC branch in the junction box.
- The MSD. The candidate is the Amphenol MINI-MSD (see the `hv-system` page). The drawing gives 690 V AC for the Bussmann fuse version, MINI-MSDM250, and no DC rating; the Zhongrong version, MINI-MSDM2501, is rated 750 V DC. Its ambient range ends at 70 °C.
- The MSD's position. It is modelled on the front pack's negative leg outside the negative contactor, where it adds little isolation beyond the contactor. Between the modules and the contactor, or mid-string, it would split the pack for service.
- The pin order of the junction box's OBC connector. The OBC/DC-DC has DC- on pin 1, so the pair crosses in the "Main HV harness" drawing.

## Acceptance criteria
- [ ] Every HV connector in the model has a part number, and the `hv-system` and `batteries` pages name the chosen parts.
- [ ] `just wiring check` passes and the site builds.

## Implementation notes
- The user decides the parts. Ask before fixing one.