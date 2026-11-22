---
id: qqxt.xb
title: Move the can-echo bootloader test application
status: review
priority: P2
created: "2026-11-22T09:10:06.384423225Z"
updated: "2026-11-22T09:19:18.581328320Z"
tags:
  - bootloader
depends_on:
  - qqxt.da
attempts: 1
---

`can_echo` → `ecus/bootloader/can-echo`, its own cargo workspace linked at the application offset, built by `just bootloader ci` and cached by the bootloader workflow.