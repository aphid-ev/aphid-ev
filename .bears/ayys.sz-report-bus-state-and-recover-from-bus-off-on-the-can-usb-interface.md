---
id: ayys.sz
title: Report bus state and recover from bus-off on the CAN/USB interface
status: open
priority: P2
created: "2026-11-24T17:15:40.457514666Z"
updated: "2026-11-24T17:15:40.457514666Z"
tags:
  - can-usb
depends_on:
  - ayys.cv
---

## Summary
The firmware reports individual bus errors (last error code) but not the controller's error state. A host cannot tell whether the dongle is error-passive or has gone bus-off, and nothing brings it back from bus-off. Send a `BusState` event on each state change, recover from bus-off automatically after 100 ms (epic decision), and drive the ERR LED from the state.

## Component
`ecus/can-usb/firmware`. Also: `site/content/wiki/can-usb.md`, a `crates/` library for the state tracking logic.

## Acceptance criteria
- [ ] A `BusState { state, tec, rec }` event is sent on every transition between ErrorActive, ErrorPassive and BusOff, and once after each start of the bus, so the host always knows the current state.
- [ ] On bus-off, the firmware waits 100 ms, restarts the controller with the current config (the FDCAN recovery sequence: clear INIT, then 128 × 11 recessive bits), and reports the resulting state. A stop request during the wait cancels the recovery.
- [ ] ERR LED: steady on in ErrorPassive and BusOff, the ~500 ms pulse on individual bus errors otherwise (from `ayys.dr`).
- [ ] Transition detection and recovery timing are host-tested: feed it a sequence of (state, tec, rec, time) and assert the events and the recovery time.
- [ ] `just can-usb ci-firmware` and `just host ci` pass, and the wiki describes bus states, recovery and the ERR LED.

## Implementation notes
- embassy-stm32's FDCAN exposes the error counters and state through the properties or the RX error return. Check what 0.6 gives (e.g. `Properties::bus_off`, `tx_error_count`, `rx_error_count`, `error_state`). If state changes are only observable by polling, poll at about 10 ms from the CAN supervisor. Do not patch embassy.
- Build on the reconfiguration task layout from `ayys.cv`, since recovery restarts the controller.
- Repeated bus-off (e.g. shorted bus) retries every 100 ms forever, reporting each time. That's fine for a tool, but don't flood the host: one BusOff and one recovered event per cycle.

## Edge cases
- ListenOnly mode never goes bus-off from its own transmissions. Make sure the logic does not assume TX.
- A host that is not reading: state events count as events for the overrun logic from `ayys.89`, and the latest state is resent once the queue has room.

## Verification
- Host tests as above. `just can-usb ci-firmware`, `just host ci`.

## Not verifiable here
Real bus-off and recovery (short CANH–CANL): the bench task.