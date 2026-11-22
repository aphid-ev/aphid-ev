---
id: ayys.qu
title: Bring in the CAN/USB dongle's KiCad project
status: open
priority: P1
created: "2026-11-22T11:01:38.870242803Z"
updated: "2026-11-22T11:01:38.870242803Z"
tags:
  - manual
  - can-usb
  - hw
---

## Summary
The dongle's KiCad project lives in another repository. Copy it into `ecus/can-usb/hardware` so `just can-usb ci-hardware` checks it and `just can-usb export` puts its schematic on the site.

## Steps
- Copy the project files into `ecus/can-usb/hardware`, keeping the project file name (KiCad keys the schematic's annotations to it).
- Move any project-specific symbols or footprints into `ecus/libraries/aphid` or keep them as a project library, and point the library tables at them.
- Run `just can-usb erc`, `just can-usb drc` and `just can-usb export`.
- Check the schematic's pins against the firmware: USB on PA11/PA12, FDCAN1 on PB8/PB9, a 16 MHz crystal on an STM32G431CB. Report any difference rather than changing either side.

## Expected result
`just can-usb ci-hardware` passes, and the README and `can-usb` wiki page point at the board.