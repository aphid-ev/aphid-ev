---
id: qqxt.6x
title: Add the boot protocol crate
status: done
priority: P2
created: "2026-11-22T09:09:55.359253030Z"
updated: "2026-11-23T19:00:21.389329Z"
tags:
  - crates
  - bootloader
depends_on:
  - u4mx
attempts: 1
---

Extract the bootloader's protocol logic into `crates/boot-protocol`: frame IDs, command decoding, response and wakeup encoding, block assembly, CRC check, error codes and the erase key. No HAL dependency, host tests for each frame and for block assembly. Done when `just host ci` passes.