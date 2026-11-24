---
id: ayys.dr
title: Drive the transceiver standby pin and the LEDs of the revA board
status: open
priority: P1
created: "2026-11-22T12:50:03.138254862Z"
updated: "2026-11-24T17:14:39.999188268Z"
tags:
  - can-usb
---

## Summary
The firmware was written for an older board without a standby line or LEDs. On revA (`ecus/can-usb/hardware`), the TJA1042's STB is pulled up, so the transceiver stays in standby and the dongle neither sends nor receives until the MCU drives STB low. Add a board module with the revA pin assignment, drive STB, and drive the three status LEDs. Also give the device a unique USB serial number so several dongles can be told apart.

## Component
`ecus/can-usb/firmware`. Also: `ecus/can-usb/README.md`, `site/content/wiki/can-usb.md`. If the LED timing goes beyond a trivial retriggerable one-shot, put it in a `crates/` library with host tests.

## Acceptance criteria
- [ ] A `board` module (or equivalent) is the only place that names pins, and states that it targets `can-usb-hw-revA`. Pins from the schematic: STB **PC13**, LED_RX **PA7**, LED_TX **PA6**, LED_ERR **PA5**, all push-pull, active high. FDCAN1 RX PB8 / TX PB9, USB PA11/PA12, HSE 16 MHz.
- [ ] STB is driven low before the FDCAN peripheral leaves init, and high again whenever the bus is stopped (that matters once the configuration task adds stop/start; until then it stays low).
- [ ] RX and TX LEDs: a retriggerable pulse of about 20 ms per received or transmitted frame, so they stay visibly lit at full bus load without one GPIO write per frame dominating the CAN tasks (e.g. the CAN tasks signal an LED task through a `Signal`).
- [ ] ERR LED: lit while the last bus error is less than about 500 ms old. The bus-state task later extends this to error-passive and bus-off.
- [ ] The USB serial number comes from the STM32's 96-bit unique ID (hex), not the literal `"1"`.
- [ ] README and wiki state the supported board revision and pins. The wiki's Hardware section lists the LEDs (RX green, TX green, ERR red, power on the 3.3 V rail) and what each means.
- [ ] `just can-usb ci-firmware` passes.

## Implementation notes
- The schematic labels are `CAN_STB`, `LED_RX`, `LED_TX` and `LED_ERR`. The LEDs go pin → resistor → anode, cathode to GND, so high is on. This task previously listed PB5 and PA0–PA2. Those pins were for an earlier draft and are wrong.
- PC13 is in the backup domain and can only sink or source a few mA (it's a low-speed pin on the G4). Driving the TJA1042's STB input is fine; set it to low speed.
- `embassy_stm32::uid::uid()` gives the unique ID. The serial string must be `'static`: format it into a `StaticCell<heapless::String<24>>` or similar.
- Keep the clock config in sync with the bootloader comment that is already there.

## Edge cases
- STB must not be released while the FDCAN pins are in an undefined state at reset. The pull-up keeps the transceiver in standby until then, which is the intended safe default.
- LED pulses must not block or delay the CAN or USB tasks.

## Verification
- `just can-usb ci-firmware`.
- Wiki and README updated as above.

## Not verifiable here
That the transceiver leaves standby and the LEDs light: the bench task of this epic.