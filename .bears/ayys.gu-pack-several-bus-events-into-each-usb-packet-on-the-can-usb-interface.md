---
id: ayys.gu
title: Pack several bus events into each USB packet on the CAN/USB interface
status: open
priority: P1
created: "2026-11-24T17:15:07.344342219Z"
updated: "2026-11-24T17:15:07.344342219Z"
tags:
  - can-usb
depends_on:
  - ayys.89
---

## Summary
The firmware writes one bus event per 64-byte bulk IN packet. An 8-byte frame with a timestamp encodes to about 15 bytes, so each packet is mostly empty, and the dongle needs one USB transaction per CAN frame. A saturated 500 kbit/s bus is roughly 4 000–8 000 frames/s. Use the protocol crate's packer to fill each packet with as many events as are queued, without adding latency when the bus is quiet.

## Component
`ecus/can-usb/firmware/src/usb.rs`. Also: `site/content/wiki/can-usb.md`, `ecus/can-usb/firmware/Cargo.toml` (profile).

## Acceptance criteria
- [ ] The write task waits for one event, then drains the RX queue with `try_receive` into the packer until the packet is full or the queue is empty, and writes the packet. An event that does not fit opens the next packet. No timer: an idle bus sends each event at once.
- [ ] When a write fails because the host disconnected, the packet's events are dropped and logged once per disconnect, not once per event.
- [ ] No packet is exactly 64 bytes if the host could be waiting for a short packet to end a transfer. Either the packer leaves room (max 63 bytes), or the firmware sends a ZLP after a full packet. Pick one, write it down in the protocol crate docs and on the wiki, and make the host task (`can-usb host throughput`) rely on the same rule.
- [ ] Release profile: measure `opt-level = "z"` against `"s"` and `3` for code size (flash is 128 KiB) and pick one. Record the size in the commit message.
- [ ] `just can-usb ci-firmware` passes.

## Implementation notes
- Packer API comes from `ayys.rv`. If it lacks something this task needs, change it in the crate with tests, and update the host in the same commit.
- embassy-stm32's USB driver has one packet buffer per endpoint: while a write waits for the host to collect it, the RX queue keeps filling. That is why `ayys.89` sizes the RX queue for ~20 ms.
- The OUT side already unpacks several frames per packet.

## Edge cases
- Bursts larger than one packet: consecutive full packets, then a short one or a ZLP per the chosen rule.
- An `Overrun` event mid-burst stays in position relative to the frames around it.

## Verification
- `just can-usb ci-firmware`. Host tests for any logic moved into the crate.
- Wiki: packets carry several events; the short-packet/ZLP rule.

## Not verifiable here
The throughput figure: the bench task.