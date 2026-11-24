---
id: ayys.da
title: Stream frames through the CAN/USB host library with transfers in flight and a fake device for tests
status: done
priority: P2
created: "2026-11-24T17:16:03.067232974Z"
updated: "2026-11-24T18:47:38.518441693Z"
tags:
  - can-usb
depends_on:
  - ayys.gu
  - ayys.cv
attempts: 1
---

## Summary
`UsbCanDevice` sends one frame per bulk OUT transfer and waits for it to complete. It submits one 64-byte IN read at a time. That round trip, not the dongle, caps throughput: an idle microframe passes between transfers. Keep several IN transfers queued and pack outgoing frames into packets with the protocol crate's packer, with several OUT transfers in flight. Put the USB transport behind a trait so the library is tested against a fake device.

## Component
`ecus/can-usb/host` (library and the `can-usb` CLI). Also: `apps/manager/src/usb.rs` only to keep it building, `site/content/wiki/can-usb.md`.

## Acceptance criteria
- [ ] IN: at least 4 transfers queued at all times (nusb's endpoint queue). Each completed packet is unpacked into an internal event queue that `receive()` serves, and a malformed packet surfaces as an error without losing the events before it.
- [ ] OUT: `send()` packs the frame into the current packet and submits it when full or when the caller flushes. A `send_all(&[Frame])` or batched API, and up to N transfers in flight, with back-pressure when N are outstanding. A frame passed to `send` is never silently lost; its failure is returned to a later call or `flush`.
- [ ] Packet boundaries follow the short-packet/ZLP rule chosen in `ayys.gu`.
- [ ] A `Transport` trait (or similar) between the library and nusb, and a fake implementation used by tests: multiple events per packet, a corrupt tail, a version mismatch at `open`, a stalled OUT (device back-pressure), a disconnect.
- [ ] The CLI's `throughput` command measures streaming: send K frames as fast as accepted in internal-loopback mode (`--mode loopback`) or against an echoing node, count received events and overruns, and print frames/s and bus load in %.
- [ ] `just host ci` passes, and the manager behaves as before.
- [ ] Wiki: the CLI commands, and the throughput measured on the bench (filled in by the bench task).

## Implementation notes
- The manager's `Bus` trait sends and receives single frames. Keep `UsbCanDevice` usable that way: `send` + implicit flush at a `receive` or at a short timer, so the bootloader flashing path doesn't wait for a full packet. The manager's own behaviour is out of scope.
- No `tokio` in the library itself if nusb's futures suffice. Keep tokio in the CLI.
- `ayys.g7` moves the test ID; coordinate with it if it lands first.

## Edge cases
- The dongle NAKs because its TX queue is full (bus with no ACKing node): `send` must block or time out with a clear error, not grow memory.
- Unplug during transfers: every queued transfer completes with an error, and the library reports one disconnect error.
- Events arrive while the caller is not calling `receive`: bounded internal queue, and the library reports its own overflow distinctly from the dongle's `Overrun`.

## Verification
- Tests against the fake transport. `just host ci`.

## Not verifiable here
Real throughput figures: the bench task.