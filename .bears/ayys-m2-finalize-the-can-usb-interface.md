---
id: ayys
title: "M2: Finalize the CAN/USB interface"
type: epic
status: open
priority: P1
created: "2026-11-22T10:51:28.197768027Z"
updated: "2026-11-22T10:51:28.197768027Z"
tags:
  - can-usb
---

## Goal
A finished dongle that the app and the flashing tool can rely on: a board, firmware and host library, verified on a real bus.

## Scope
- The board in `ecus/can-usb/hardware`, a first hardware revision tagged `can-usb-hw-revA`.
- The firmware and `can-usb-host`: bit rate configurable rather than fixed at 500 kbit/s, bus-off handling, and the CLI's test frames moved off the bootloader's IDs.
- The `can-usb` wiki page brought up to date.

## Decisions
- The USB wire format in `crates/can-usb-protocol` stays compatible unless a change is needed for bit-rate configuration, which would be a breaking change to both sides at once.

## Open questions
- Does a KiCad project for the dongle exist outside this repository, or does this epic include designing the board?

## Safety
None.

## Acceptance
- [ ] `just can-usb ci` passes, including ERC and DRC on the board.
- [ ] The host library is tested against a fake device.

## Verified on hardware
- `can-usb listen` and `throughput` against a second node at 500 kbit/s, and recovery after bus-off.
- Enumerates without driver setup on Linux (with the udev rule), Windows and macOS.