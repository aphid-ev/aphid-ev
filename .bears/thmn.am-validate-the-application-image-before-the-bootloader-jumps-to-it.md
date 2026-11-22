---
id: thmn.am
title: Validate the application image before the bootloader jumps to it
status: open
priority: P1
created: "2026-11-22T09:23:05.885427495Z"
updated: "2026-11-22T10:52:54.174998086Z"
tags:
  - bootloader
  - crates
  - safety
---

The bootloader starts whatever is in the application region after the wakeup window, including erased flash or a half-written image, which `ecus/bootloader/AGENTS.md` forbids. Design an image header or trailer (length, CRC, version) checked by logic in `crates/boot-protocol` with host tests, and stay in the bootloader when it fails. This changes the flash layout and every application's `memory.x`, and it gives the wakeup frame a real version in place of the hard-coded 1.0.0. Update the wiki page `bootloader`.