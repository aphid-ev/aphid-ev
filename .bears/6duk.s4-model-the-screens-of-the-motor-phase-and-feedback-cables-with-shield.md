---
id: 6duk.s4
title: Model the screens of the motor phase and feedback cables with `shield`
status: open
priority: P3
created: "2026-11-21T22:30:30.134017117Z"
updated: "2026-11-22T10:52:54.140688337Z"
tags:
  - wiring
---

wirebug 0.2.0 can model a cable or twisted-pair screen with `shield [<ends>];` and draws its drain in harness views. Two cables in `wiring/systems/traction.wb` are typed as shielded but have no `shield` line: `motor_phases` ("Shielded 3-core") and `motor_feedback` ("Shielded 4x twisted pair").

Needs from the user: where each drain is terminated (which inverter or motor pin, both ends, or bonded at a gland or cut back), and whether the feedback pairs are individually screened.

Done when both cables carry `shield` lines matching the build, the harness views show the drains, and `just wiring check` passes.