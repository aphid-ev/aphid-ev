---
id: qqxt.da
title: Move the bootloader onto the boot protocol crate
status: review
priority: P2
created: "2026-11-22T09:10:01.412141309Z"
updated: "2026-11-22T09:18:26.577949981Z"
tags:
  - bootloader
depends_on:
  - qqxt.6x
attempts: 1
---

`can_bootloader` → `ecus/bootloader`, keeping only clock, flash, CAN and jump code and using `crates/boot-protocol` for everything on the wire. Wire behaviour unchanged. List both chips in the justfile's `families`. Report binary size before (5412 B G431, 5416 B G491) and after. Done when `just bootloader ci` passes.