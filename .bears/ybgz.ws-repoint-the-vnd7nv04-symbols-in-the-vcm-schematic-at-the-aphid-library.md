---
id: ybgz.ws
title: Repoint the VND7NV04 symbols in the VCM schematic at the aphid library
status: open
priority: P3
created: "2026-11-23T19:00:57.574139Z"
updated: "2026-11-23T19:00:57.574139Z"
tags:
  - manual
  - vcm
  - hw
---

Found in the review of ncmh.a6. `ecus/vcm/hardware/digital.kicad_sch` (around lines 1491, 1662, 1903) refers to `aphid-vehicle-controller:VND7NV04`. No library table defines that nickname; the symbol lives in `aphid`. It renders from the cached copy, but "update symbols from library" won't find it. Fix in KiCad with Change Symbols, not by hand-editing the file. May be moot if ybgz.9k replaces these parts.