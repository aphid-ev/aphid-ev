---
id: ayys.dz
title: Verify the CAN/USB interface firmware on the revA board
status: open
priority: P2
created: "2026-11-24T17:16:03.106762257Z"
updated: "2026-11-24T17:32:41.711849975Z"
tags:
  - can-usb
  - manual
depends_on:
  - ayys.sz
  - ayys.gu
  - ayys.da
  - ayys.zm
---

## Summary
Check on a revA board what the agent tasks of epic `ayys` could only build and host-test: the standby pin and LEDs, timestamps, ordering and loss under full load, configuration, bus-off recovery and enumeration.

## Steps
Global options come before the command and take kbit/s: `can-usb -b 500 -m loopback throughput`.

1. Set the option bytes once if this is a fresh board (wiki, Option bytes). Then `just can-usb flash`. Reflash the echo node with the new `can-echo` (`just bootloader flash-echo-can`): it now answers `0x7cf` on `0x7ce`.
2. `cargo run -p can-usb-host -- info` on Linux. Repeat enumeration on Windows and macOS.
3. Loopback, no bus needed: `can-usb -m loopback throughput`.
4. Against an ECU running `can-echo` at 500 kbit/s, bus terminated at both ends: `can-usb ping`, then `can-usb throughput`.
5. With two revA dongles A and B on a terminated bus with the echo node, at 500 kbit/s: A runs `can-usb listen` while B runs `can-usb throughput`.
6. Bit rates, with two dongles and no echo node (`can-echo` is fixed at 500 kbit/s): for each of `-b 125`, `-b 250` and `-b 1000`, A runs `listen` and B runs `throughput`, both at the same rate.
7. Listen-only: with only A and B on the bus, A runs `can-usb -m listen-only listen` while B runs `can-usb throughput`.
8. Short CANH to CANL for a second while `listen` runs and another node sends, then release.
9. Watch the LEDs throughout. Optionally leave `listen` running past 72 minutes of traffic.

## Expected result
- `info` shows the protocol version, firmware version and a serial that differs between two boards, and matches the USB serial string. No driver install on any OS (Linux with the udev rule).
- Loopback: zero overruns and zero frames out of sequence. Record frames/s on the wiki's Throughput table.
- Against `can-echo`: every ping is answered on `0x7ce`. Throughput shows no bus errors, no bus-off, zero out of sequence and zero overruns. The reported load is about 50 %, because it counts only the echoes; together with the dongle's own frames the bus is near 100 %. Record frames/s.
- Two dongles: A's `listen` shows B's sequence numbers in order with no gaps, times rising, and no overrun.
- Each bit rate: same as above at that rate.
- Listen-only: A shows B's frames, does not ACK them, and B reports acknowledge errors and goes error-passive.
- Short: BusOff is reported once or twice per ~100 ms while shorted, then ErrorActive about 100 ms after release. ERR LED steady during bus-off.
- RX/TX LEDs flash with traffic and look lit at full load. ERR pulses on bus errors. The power LED is on whenever USB is plugged in.
- Past 72 minutes, the time `listen` prints keeps rising past 4294967295 µs.
