---
id: 6duk.vk
title: Report the harness rail drawing defects to wirebug
status: open
priority: P3
created: "2026-11-23T18:12:37.865440Z"
updated: "2026-11-23T18:12:37.865440Z"
tags:
  - wiring
---

## Summary
wirebug 0.2.1 draws two things badly in harness views. They are wirebug's to fix; the model does not work around them.

- A conductor between two connectors in the same column is drawn into the top or bottom edge of the boxes instead of to its pins. It shows on the stop-lamp rail in "Rear LV harness", the inlet ground rail in "Charge signal harness" and the OBC ground lead in "Powertrain signal harness".
- A wire label near the right edge is clipped by the canvas. It shows on the inlet ground label in "Charge signal harness".
- A harness view draws every conductor between its connectors. There is no way to leave a cable out, so "CAN harness" also carries the whole tunnel cable and the EV-signal lead.

## Steps
- Open issues on LHelge/wirebug with a minimal `.wb` example for each.
- When a release fixes them, bump `wirebug_version`, re-render and look at the three views.

## Acceptance criteria
- [ ] The issues exist upstream and are linked here.