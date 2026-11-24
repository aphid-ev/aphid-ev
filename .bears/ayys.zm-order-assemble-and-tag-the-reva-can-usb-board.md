---
id: ayys.zm
title: Order, assemble and tag the revA CAN/USB board
status: open
priority: P1
created: "2026-11-24T17:32:34.303021120Z"
updated: "2026-11-24T17:32:34.303021120Z"
tags:
  - can-usb
  - hw
  - manual
---

## Summary
The revA layout is finished but no board exists yet. The bench check `ayys.dz` needs a built board.

## Steps
1. `just can-usb ci-hardware` (ERC and DRC) on the commit you send to fabrication.
2. Generate the fabrication outputs from that commit and order the board. Outputs are not committed.
3. Tag that exact commit `can-usb-hw-revA`.
4. Assemble one or more boards. Check the 3.3 V rail and the power LED before connecting a probe.
5. Set the option bytes (wiki, Option bytes) so PB8/BOOT0 doesn't start the ROM bootloader.

## Expected result
A powered revA board with 3.3 V ± 3 %, the power LED on, and the STM32G431 reachable over SWD on the STDC14 connector.