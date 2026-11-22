---
id: bykf
title: Give each ECU's bootloader its own node ID
status: open
priority: P2
created: "2026-11-22T09:23:05.901169192Z"
updated: "2026-11-22T09:23:05.901169192Z"
tags:
  - bootloader
---

`NODE_ID` in `ecus/bootloader/src/main.rs` is 0 for every build, so every ECU answers the same catch and command frames and only one can be on the bus while flashing. Decide where the ID comes from (a build feature per ECU, a value in flash, strap pins) and define the assignment per ECU in a shared crate.