---
id: qqxt.xb
title: Move the can-echo bootloader test application
status: done
priority: P2
created: "2026-11-22T09:10:06.384423225Z"
updated: "2026-11-23T19:00:21.418067Z"
tags:
  - bootloader
depends_on:
  - qqxt.da
attempts: 1
---

`can_echo` → `ecus/bootloader/can-echo`, its own cargo workspace linked at the application offset, built by `just bootloader ci` and cached by the bootloader workflow.