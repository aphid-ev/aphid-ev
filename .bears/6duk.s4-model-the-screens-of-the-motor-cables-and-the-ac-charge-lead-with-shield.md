---
id: 6duk.s4
title: Model the screens of the motor cables and the AC charge lead with `shield`
status: open
priority: P3
created: "2026-11-21T22:30:30.134017117Z"
updated: "2026-11-23T18:12:48.413023Z"
tags:
  - wiring
---

wirebug can model a cable or twisted-pair screen with `shield [<ends>];` and draws its drain in harness views. Two cables in `wiring/systems/traction.wb` are typed as shielded but have no `shield` line: `motor_phases` ("Shielded 3-core") and `motor_feedback` ("Shielded 4x twisted pair").

The AC charge lead `ac_charge` in `wiring/looms/ac_charge.wb` is a plain 3-core. The `obc` wiki page says Mitsubishi screens the line and neutral conductors and earths the screens to the body, so decide whether this lead is screened too.

Needs from the user: where each drain is terminated (which inverter or motor pin, both ends, or bonded at a gland or cut back), whether the feedback pairs are individually screened, and whether the AC lead is screened.

Done when the cables carry `shield` lines matching the build, the harness views show the drains, and `just wiring check` passes.