---
id: ncmh.fv
title: Point the wiki at the monorepo for the VCM and document EV-CAN changes
status: open
priority: P2
created: "2026-11-22T10:15:23.181428601Z"
updated: "2026-11-22T10:15:23.181428601Z"
tags:
  - site
depends_on:
  - ncmh.z9
  - ncmh.pg
---

Replace links to the vehicle-controller repository on `vcm`, `main-firmware` and `monitor-firmware`, and note on `ev-can` which messages `crates/ev-can` defines beyond those the page documents. Done when `just site build` passes.