---
id: 6duk.qm
title: Define the parameter model and protocol
status: open
priority: P1
created: "2026-11-22T10:53:27.339508961Z"
updated: "2026-11-22T10:53:27.339508961Z"
tags:
  - crates
---

## Summary
A shared crate for configuration: how a parameter is identified, typed, bounded and stored in flash, and the CAN protocol the manager uses to list, read and write them on any ECU.

## Component
A new crate in `crates/`, and a wiki page for the protocol.

## Acceptance criteria
- [ ] Parameters carry a type, unit, default and bounds; writes outside the bounds are refused by the ECU.
- [ ] The protocol has encode and decode functions with pinned-bytes tests and a round trip against a fake ECU.
- [ ] `just host ci` passes.

## Edge cases
- An app talking to firmware with more or fewer parameters than it knows.
- Flash wear and a power loss during a write.

## Verification
`just host ci`; a wiki page documents the protocol.