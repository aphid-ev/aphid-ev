---
id: ayys.rv
title: Redesign the CAN/USB wire protocol for batching, timestamps and configuration
status: open
priority: P1
created: "2026-11-24T17:14:21.334397364Z"
updated: "2026-11-24T17:14:21.334397364Z"
tags:
  - crates
  - can-usb
---

## Summary
Make `crates/can-usb-protocol` the complete, single definition of the USB protocol between the dongle and the host, and break the wire format once to get there (epic `ayys`, Decisions). It gains the constants both sides now duplicate, a protocol version, the new bus events with timestamps, the control request types for configuration, and a packet packer/unpacker shared by firmware and host. The firmware, the host library and the manager are ported in the same commit so every workspace builds and both ends speak the new format. Behaviour beyond what is needed to keep them working (batching, control-request handling, bus state) comes in later tasks.

## Component
`crates/can-usb-protocol`. Also allowed: `ecus/can-usb/firmware`, `ecus/can-usb/host`, `apps/manager/src/usb.rs`, `site/content/wiki/can-usb.md`, `ecus/can-usb/AGENTS.md`.

## Acceptance criteria
- [ ] Constants: `VENDOR_ID`, `PRODUCT_ID`, `BULK_OUT_EP`, `BULK_IN_EP`, `MAX_PACKET_SIZE` (64), `PROTOCOL_VERSION`, and the vendor control request codes. Firmware `usb.rs` and host `device.rs` use them, and no literal copy remains.
- [ ] Host→device: `Frame` (unchanged validation: ID range, ≤ 8 bytes).
- [ ] Device→host `BusEvent`: `Frame { timestamp_us: u32, frame }`, `BusError { timestamp_us, error }`, `BusState { timestamp_us, state: ErrorActive | ErrorPassive | BusOff, tec: u8, rec: u8 }`, `Overrun { timestamp_us, lost: u16 }` (or an equivalent shape, documented).
- [ ] Control types: `DeviceInfo { protocol_version, firmware_version (major, minor, patch), serial }` returned by a GET request, `Config { bitrate: Bitrate, mode: Mode }` with `Bitrate` ∈ {125k, 250k, 500k, 1M} and `Mode` ∈ {Normal, ListenOnly, InternalLoopback}, plus start/stop bus requests. `Config::default()` is 500 kbit/s, Normal.
- [ ] Packer: appends postcard-encoded values to a `MAX_PACKET_SIZE` buffer, reports when a value does not fit (the caller then flushes and retries), and never splits a value. Unpacker: iterates every value in one packet with `take_from_bytes`, yielding an error for a malformed tail without panicking. Both are `no_std` without alloc. `postcard` becomes a dependency of the crate.
- [ ] Each type keeps a `wire_encoding_is_stable` test pinning its bytes, and the packer and unpacker have round-trip and edge-case tests.
- [ ] Firmware ported: sends `BusEvent::Frame` with a timestamp from the FDCAN envelope, uses the crate's constants and the unpacker for OUT. It may still send one event per IN packet.
- [ ] Host ported: `receive` unpacks every event in a packet (buffering the extras) instead of `from_bytes`, which drops all but the first. The manager's `UsbBus` and the CLI build and behave as before.
- [ ] The crate builds for `thumbv7em-none-eabi` and on the host with no features other than `defmt`.

## Implementation notes
- Today `BusEvent` derives `Ord` so the priority channels sort it. Drop that: the next task replaces the priority channels with FIFOs, and ordering by ID is wrong for received frames. Keep `Frame`'s `Ord` only if something still uses it.
- `Frame.dlc` is `usize`. Consider `u8` while the format is open anyway.
- Timestamps: embassy-stm32's FDCAN `Envelope.ts` is an `embassy_time::Instant`. At the firmware's `tick-hz-32_768` that is about 30 µs resolution. Raising the tick rate to 1 MHz is acceptable if the build stays clean; say which you chose on the wiki.
- Put the firmware version in `DeviceInfo` from `CARGO_PKG_VERSION`, parsed at compile time.
- Keep `defmt` derive behind the existing feature.
- Remove the `control_in`/`control_out` placeholders (requests 100/101) from the host. The real control requests arrive with the configuration task.
- Update `ecus/can-usb/AGENTS.md` if a rule in it changes (e.g. the take_from_bytes note now points at the unpacker).

## Edge cases
- A packet that ends in the middle of a value (corrupt or truncated): the unpacker returns the values before it and an error, and the firmware logs and drops the rest of the packet.
- An empty packet (ZLP) unpacks to nothing.
- An event larger than a packet must be impossible. Add a test asserting the largest event's encoded size fits `MAX_PACKET_SIZE`.
- Unknown enum discriminants from a newer peer fail to decode rather than misparse.

## Verification
- `cargo test -p can-usb-protocol`, `just host ci`, `just can-usb build`.
- Rewrite the USB protocol section of `site/content/wiki/can-usb.md`: constants, packet rule, every type's encoding with one worked example each, the version, and the control requests. Describe the control requests as part of the protocol; the configuration task implements them in the firmware.
- Commit as `feat(crates)!:` with a `BREAKING CHANGE:` footer: reflash the dongle and update the host library together.

## Not verifiable here
Real timestamps and enumeration: bench task of this epic.