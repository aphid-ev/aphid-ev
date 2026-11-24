---
id: ayys.g7
title: Move the can-usb CLI's ping off the bootloader wakeup ID
status: open
priority: P3
created: "2026-11-22T09:13:03.308542875Z"
updated: "2026-11-24T17:16:03.142305982Z"
tags:
  - can-usb
depends_on:
  - ayys.rv
---

`can-usb ping` and `throughput` send on `0x7ff`, which the bootloader uses for its wakeup frame. Harmless while the bootloader only sends that ID, but a poor test identifier on a bus with ECUs. Pick an ID outside the boot protocol's range and define it in a shared crate.