---
id: ayys
title: "M2: Finalize the CAN/USB interface"
type: epic
status: open
priority: P1
created: "2026-11-22T10:51:28.197768027Z"
updated: "2026-11-24T17:14:00.115857657Z"
tags:
  - can-usb
---

## Goal
A finished dongle that the manager and the flashing tool can rely on: a board, firmware and host library, verified on a real bus. It passes a fully loaded 500 kbit/s bus to the computer without losing or reordering frames, timestamps what it receives, and is configured (bit rate, mode) from the host.

## Scope
- `crates/can-usb-protocol`: a breaking redesign of the USB wire format, usable unchanged by the firmware, the host library and the manager.
- `ecus/can-usb/firmware`: pins of the revA board (transceiver standby, LEDs), restructured CAN/USB data path, throughput, configuration over control requests, bus-state reporting and bus-off recovery.
- `ecus/can-usb/host`: kept in lockstep with the protocol, plus host-side throughput (transfers in flight, batched sends) and the CLI.
- `apps/manager`: only what it needs to keep building against the new host library. No manager features.
- The `can-usb` wiki page, updated by each task that changes what it describes.
- Out of scope: CAN FD, the manager UI, a CAN bootloader for the dongle (it stays at `0x08000000`, see `yng2`), the debug UART on PB6/PB7.

## Decisions
- **Wire format breaks once** in this epic. The existing dongle is reflashed together with the new host library. After that, a `PROTOCOL_VERSION` that the host reads at open makes later mismatches an error instead of a misparse.
- **One source for the protocol.** VID/PID `0xc0de:0xf055`, bulk OUT `0x01`, bulk IN `0x82`, the 64-byte packet size, control request codes and the version live in `can-usb-protocol` as constants. Firmware and host import them. The values themselves do not change.
- **Packets are self-contained.** Every bulk packet (≤ 64 bytes) holds a whole number of postcard values back to back. No value spans packets. The crate provides the packer and the unpacker that both sides use.
- **Data path:** host→device bulk OUT carries frames to send. Device→host bulk IN carries bus events: received frame (with timestamp), bus error, bus state change, receive overrun (count of frames lost).
- **Configuration and queries go over vendor control requests**, not the bulk pipes: device info (protocol version, firmware version, serial), set config (bit rate, mode), start and stop the bus. Bit rates: 125, 250, 500 and 1000 kbit/s, classic CAN. Modes: normal, listen-only, internal loopback. 500 kbit/s normal is the power-on default, so a host that never configures still works.
- **Timestamps:** each received frame and each bus event carries a u32 microsecond timestamp from the dongle's clock (wraps after about 71 minutes; the host unwraps).
- **Order is preserved.** Received frames reach the host in bus order. Frames from the host go out in the order sent. A full transmit queue pushes back over USB (NAK), and no queued frame is evicted.
- **Loss is reported, never silent.** When the dongle cannot forward a received frame, it counts the loss and sends an overrun event.
- **Bus-off:** reported as a bus-state event. The dongle recovers automatically after 100 ms and reports the state again. A host can also stop and restart the bus by control request.
- **Board:** only `can-usb-hw-revA` (the board in `ecus/can-usb/hardware`) is supported. Pins from the schematic: USB PA11/PA12, FDCAN1 RX PB8 / TX PB9, TJA1042 STB on PC13 (pulled up, active high = standby), LED_RX PA7, LED_TX PA6, LED_ERR PA5 (all active high), 16 MHz HSE on PF0/PF1. The older board without STB and LEDs is retired.
- The receive filter accepts standard and extended identifiers.

## Safety
None. The dongle is a tool. Listen-only mode exists so it can be connected to the car without transmitting or acknowledging.

## Acceptance
- [ ] `just host ci`, `just can-usb ci-firmware` and `just can-usb ci-hardware` pass.
- [ ] The protocol crate has host tests for every type's encoding (pinned bytes), the packer and the unpacker, and the host library is tested against a fake device.
- [ ] No constant of the USB protocol is defined outside `can-usb-protocol`.
- [ ] The `can-usb` wiki page describes the new protocol, the LEDs, the configuration requests and the board revision.

## Verified on hardware
- Enumerates on Linux (udev rule), Windows and macOS without driver setup, and reports its protocol version and serial.
- `can-usb throughput` in internal loopback and against a second node at 500 kbit/s reaches bus saturation with zero overruns.
- `can-usb listen` on a fully loaded bus shows monotonic timestamps and no reordering.
- Bus-off (CANH shorted to CANL, then released) is reported and recovered from.
- LEDs: RX/TX flash with traffic, ERR lights on errors and bus-off. In listen-only mode the dongle does not ACK.