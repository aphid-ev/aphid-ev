---
id: ayys.ep
title: Stamp received CAN frames with their arrival time on the CAN/USB interface
status: open
priority: P3
created: "2026-11-24T17:47:20.725700526Z"
updated: "2026-11-24T18:58:28.826814352Z"
tags:
  - can-usb
depends_on:
  - ayys.cv
attempts: 1
---

## Summary
Received frames are stamped by embassy-stm32's FDCAN `Envelope.ts`. In embassy-stm32 0.6.0 the timestamp counter counts CAN bit times (prescaler 1, RM0440 TSCC.TCP), but `calc_ns_per_timer_tick` converts it as if it counted FDCAN clock quanta (`1e9 / (f_fdcan * (nbrp+1))`), so the arrival correction is about 1000 times too small. In practice the stamp is the moment `rx_task` reads the frame out of the RX FIFO (found in the review of `ayys.rv`).

Compute the arrival time in the firmware from the RX element's 16-bit timestamp and the TSCV counter at read time (bit time from the current `Config`), or report the conversion upstream to embassy and use a fixed release. Do not patch or vendor embassy here.

## Component
`ecus/can-usb/firmware`. The conversion (counter delta, bit time, 16-bit wrap) goes in a `crates/` library with host tests.

## Acceptance criteria
- [ ] A frame's `timestamp_us` is its arrival time within one bit time plus the read latency of the counter, at every supported bit rate.
- [ ] Host tests for the conversion, including counter wrap and a frame that waited longer than one counter period (65 536 bit times, 131 ms at 500 kbit/s). Document the latter as the limit.
- [ ] The wiki's timestamp paragraph says what the stamp means.
- [ ] `just can-usb ci-firmware`, `just host ci`.

## Not verifiable here
Accuracy on a bus: compare against a second node's or a logic analyser's timing (bench).

## Decision needed
embassy's public API cannot give one-bit-time accuracy. `Envelope.ts` subtracts `20 ns · Δ` rounded up to whole microseconds, so inverting it pins Δ only to within about 50 bit times, and the driver does not return the `now` it used. Choose one route:

1. **PAC read in the firmware.** Before `can_rx.read()`, read RXF0S (fill level, get index), the RXTS field (bits 15:0 of word R1) of the RX FIFO 0 element at that index, TSCV and `Instant::now()`. Arrival = now − Δ · bit time. The firmware is the only reader in non-buffered mode, so `read()` then takes that same element. Caveat: this widens the PAC use from ayys.sz to message RAM and duplicates embassy's message RAM layout (18-word elements after the filters, or the start address from RXF0C.F0SA).
2. **Fix upstream.** Report to embassy-stm32 that `calc_ns_per_timer_tick` runs before `apply_config` writes NBTP and TSCC, and that the counter counts bit times. The tick is `nbrp · (1 + seg1 + seg2) · (tcp + 1) / f_fdcan`, taken from the config. Bump to a release with the fix and keep `Envelope.ts`.
