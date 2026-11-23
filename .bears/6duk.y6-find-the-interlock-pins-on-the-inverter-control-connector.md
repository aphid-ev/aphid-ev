---
id: 6duk.y6
title: Find the interlock pins on the inverter control connector
status: open
priority: P2
created: "2026-11-23T18:12:37.812405Z"
updated: "2026-11-23T18:12:37.812405Z"
tags:
  - wiring
  - site
  - manual
---

## Summary
The inverter's lid interlock is part of the HV interlock loop, but its pins on the control connector are not known. `wiring/components/hv/inverter.wb` keeps `interlock_in` and `interlock_out` outside the connector, and the `inverter` wiki page lists the pins as TBD.

## Steps
- Find the two interlock pins, from a Leaf wiring diagram or by measuring continuity on the unit with the lid on and off.
- Give the two ports their pins in `inverter.wb` and update the pin table on the `inverter` page.

## Acceptance criteria
- [ ] Both interlock ports are on the control connector with pin numbers and a source.
- [ ] The interlock leads to the inverter appear in a harness view.
- [ ] `just wiring check` passes and the site builds.