---
id: ayys.ep
title: Stamp received CAN frames with their arrival time on the CAN/USB interface
status: open
priority: P3
created: "2026-11-24T17:47:20.725700526Z"
updated: "2026-11-24T17:47:24.015683001Z"
tags:
  - can-usb
depends_on:
  - ayys.cv
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