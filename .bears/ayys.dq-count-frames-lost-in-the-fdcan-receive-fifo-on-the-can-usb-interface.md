---
id: ayys.dq
title: Count frames lost in the FDCAN receive FIFO on the CAN/USB interface
status: open
priority: P3
created: "2026-11-24T18:00:34.717743188Z"
updated: "2026-11-24T18:00:34.717743188Z"
tags:
  - can-usb
---

## Summary
The firmware counts frames lost to a full event queue (`OverrunCounter`, task `ayys.89`), but not frames lost inside the FDCAN's 3-element RX FIFO 0: embassy-stm32 0.6 exposes no API for the message-lost flag (IR.RF0L). The epic says loss is never silent, so this gap should close.

**Decision needed from the user before starting:** either (a) ask embassy upstream for a message-lost API and wait for a release, or (b) enable embassy-stm32's `unstable-pac` feature and read and clear IR.RF0L (write 1 to clear) from the firmware alongside the driver. Option (b) touches a driver-owned register. Do not patch or vendor embassy.

## Component
`ecus/can-usb/firmware`. Counting reuses `OverrunCounter::record` in `crates/can-usb-protocol`.

## Acceptance criteria
- [ ] A frame lost in the hardware FIFO is counted into the next `Overrun` event.
- [ ] The wiki's CAN side section no longer says these losses go uncounted.
- [ ] `just can-usb ci-firmware`.

## Not verifiable here
Forcing a hardware FIFO overflow needs a board (e.g. stall `rx_task` under bus load).