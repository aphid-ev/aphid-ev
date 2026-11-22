---
id: thmn
title: "M3: Finalize the bootloader and the flashing tool"
type: epic
status: open
priority: P1
created: "2026-11-22T10:51:40.495571138Z"
updated: "2026-11-22T10:51:40.495571138Z"
tags:
  - bootloader
  - crates
  - manager
depends_on:
  - 6duk
  - ayys
---

## Goal
Any ECU can be reflashed over CAN through the dongle from a computer, and a bad or interrupted update never leaves it unrecoverable.

## Scope
- The bootloader for every MCU family chosen in M1, one cargo feature each.
- Image validation before the jump: an image header or trailer with length, CRC and version, which fixes the flash layout every application links against.
- Node IDs per ECU from M1.
- The programming device's side of the protocol as a UI-free library in the manager's core, with a CLI over `can-usb-host`, tested against a fake bus.
- The `bootloader` wiki page, with its tables generated from `crates/boot-protocol`.

Out of scope: the GUI for updates (M4).

## Decisions
- The protocol is frozen once a bootloader is in an ECU. Extend it compatibly.
- Bootloaders are flashed with a probe; applications over CAN.

## Safety
An ECU must stay in the bootloader when its application fails validation, so it can always be reflashed. The bootloader never erases its own region.

## Acceptance
- [ ] `just bootloader ci` builds every family.
- [ ] Image validation and the update sequence are covered by host tests, including an interrupted update.
- [ ] The flash layout is the same in the bootloader, every application's `memory.x` and the image tool.

## Verified on hardware
- On every MCU family: flash `can-echo` over CAN with the CLI, see it echo, then interrupt an update halfway and recover by reflashing.