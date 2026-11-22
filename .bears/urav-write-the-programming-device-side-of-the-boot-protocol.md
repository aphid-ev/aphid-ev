---
id: urav
title: Write the programming device side of the boot protocol
status: open
priority: P2
created: "2026-11-22T09:23:11.662504263Z"
updated: "2026-11-22T09:23:11.662504263Z"
tags:
  - manager
  - crates
---

Nothing sends the programming device's frames yet, so no ECU can be flashed over CAN. `crates/boot-protocol` already encodes catch, commands and block data and decodes responses. Add the update sequence (catch, erase, blocks with CRC, start) as a UI-free library over a CAN transport trait, tested against a fake bus, with a CLI on top of `can-usb-host`. Test it on a bench against `can-echo` (manual step).