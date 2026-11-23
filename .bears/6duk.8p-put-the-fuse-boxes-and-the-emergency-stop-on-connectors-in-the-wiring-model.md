---
id: 6duk.8p
title: Put the fuse boxes and the emergency stop on connectors in the wiring model
status: open
priority: P1
created: "2026-11-23T18:12:37.763020Z"
updated: "2026-11-23T18:12:37.763020Z"
tags:
  - wiring
  - site
---

## Summary
A harness view only draws a conductor whose both ends are on connectors. The primary fuse box, the rear EV fuse box, the 12 V battery and the emergency stop have none in the model, so every fused 12 V feed, both relay control lines, the heavy battery and DC-DC leads and the interlock lead from the emergency stop into the front pack are missing from the harness drawings. `undrawn_conductor` does not flag them.

## Component
`wiring/components/lv/primary_fuse_box.wb`, `ev_fuse_box.wb`, `e_stop.wb`, `lv_battery.wb`, the harness views in `wiring/looms/`, and the `cab-loom` and `rear-loom` wiki pages.

## Needs from the user
- The pin order of the four-way FCI APEX 2.8 connector on the Littelfuse 880073.
- The cavity layout of the Eaton 15304-2-2-4: which fuse and relay sits where, and the check on the 2D print that the relay stud does not also feed pin 30.
- The emergency stop part and its terminals.

## Acceptance criteria
- [ ] Every port of both fuse boxes, the battery and the emergency stop is on a connector or stud with a pin.
- [ ] Every fused feed, relay control line and interlock lead appears in a harness view.
- [ ] The notes on the `cab-loom`, `rear-loom` and `wiring` pages that say these leads are not drawn are removed.
- [ ] `just wiring check` passes and the site builds.

## Verification
`just wiring check`, `just wiring render`, `just site build`.