---
id: an5b
title: Split the CAN/USB firmware logic out of the wire protocol crate
status: proposed
priority: P3
created: "2026-11-24T19:18:40.245493362Z"
updated: "2026-11-24T19:18:40.294664010Z"
tags:
  - crates
  - can-usb
---

`crates/can-usb-protocol` now holds both the wire format (constants, `Frame`, `BusEvent`, control payloads, `Packer`/`unpack`, `TimestampUnwrapper`) and the dongle's firmware behaviour (`Controller`, `Batcher`/`DropTracker`, `OverrunCounter`, `BusMonitor`), all public. The desktop app only needs the first half. Consider moving the second half into a `can-usb-core` crate used only by the firmware, so the protocol crate the manager depends on stays small and its public API is only the contract. Raised in the review of epic `ayys`.

Checks: `just host ci`, `just can-usb ci-firmware`.