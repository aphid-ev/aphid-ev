---
id: kuzv
title: Drive the transceiver standby pin and the LEDs on the CAN/USB board
status: open
priority: P1
created: "2026-11-22T12:50:03.138254862Z"
updated: "2026-11-22T12:50:03.138254862Z"
tags:
  - can-usb
---

The CAN/USB firmware was written for a board without these signals. On the new board (`ecus/can-usb/hardware`):

- **PB5 → TJA1042 STB.** STB has an internal pull-up, so the transceiver sits in standby until the MCU drives PB5 low. Without this the interface neither sends nor receives normal traffic. Drive it low before the FDCAN driver starts.
- **LEDs, active high through series resistors:** PA0 RX (green), PA1 TX (green), PA2 error (red). The power LED is on the 3.3 V rail and needs no firmware.
  - RX/TX: pulse on each received/transmitted frame, long enough to see at bus load (e.g. ~20 ms one-shot, retriggerable).
  - Error: lit on bus errors / error-passive / bus-off.

Keep the blink timing logic in a host-testable place if it grows beyond trivial, per the repo rule.

Update the Hardware section of `site/content/wiki/can-usb.md` with the LED meanings.

## Acceptance

- `just can-usb ci-firmware` passes.
- PB5 is driven low at startup; LEDs are wired to the events above.
- Wiki page lists the LEDs.