---
id: 6duk.n3
title: Define the CAN2 messages and node IDs
status: open
priority: P1
created: "2026-11-22T10:53:27.324238745Z"
updated: "2026-11-22T10:53:27.324238745Z"
tags:
  - crates
  - safety
depends_on:
  - 6duk.yw
---

## Summary
A shared crate for the DIY bus: the node ID of every ECU, and the messages between the VCM and the BMS controllers (pack status, limits, contactor requests and states, faults), with periods and timeouts from the contactor specification.

## Component
A new crate in `crates/`, and a CAN2 section on the `ev-can` wiki page or a new page.

## Acceptance criteria
- [ ] Every message has an encoder, a decoder and a pinned-bytes test.
- [ ] Node IDs do not collide with the boot protocol's ranges (`0x7d0`-`0x7ff`).
- [ ] `just host ci` passes.

## Edge cases
- Units running different firmware versions on the same bus.

## Verification
`just host ci`; the wiki page lists the messages.