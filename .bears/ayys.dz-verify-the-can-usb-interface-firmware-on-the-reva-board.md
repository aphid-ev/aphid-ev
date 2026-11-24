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
1. Set the option bytes once if this is a fresh board (wiki, Option bytes). Then `just can-usb flash`.
2. `cargo run -p can-usb-host -- info` on Linux. Repeat enumeration on Windows and macOS.
3. Loopback: `can-usb throughput --mode loopback --bitrate 500000`.
4. Connect a second node (an ECU running `can-echo`, or a second dongle), terminate the bus at both ends. Run `throughput` at 500 kbit/s and `listen` while the other node sends at full load.
5. `--bitrate 125000`, `250000` and `1000000`: `ping` against the other node at each.
6. `--mode listen-only` while the other node sends with no third node on the bus.
7. Short CANH to CANL for a second while `listen` runs, then release.
8. Watch the LEDs throughout.

## Expected result
- `info` shows the protocol version, firmware version and a serial that differs between two boards. No driver install on any OS (Linux with the udev rule).
- Loopback and two-node throughput reach at least 95 % bus load at 500 kbit/s with zero `Overrun` events. Record frames/s on the wiki.
- `listen`: timestamps strictly increasing (allowing for wrap), and the sending node's sequence counter arrives in order with no gaps.
- Each bit rate works against a node configured for it.
- Listen-only: the dongle shows the frames, and the sender reports ACK errors (nobody ACKs).
- Short: BusOff is reported, then ErrorActive again about 100 ms after release. ERR LED steady during bus-off.
- RX/TX LEDs flash with traffic and look lit at full load. The power LED is on whenever USB is plugged in.