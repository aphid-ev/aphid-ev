---
id: ncmh.z9
title: Add the Nissan EV-CAN message crate
status: done
priority: P2
created: "2026-11-22T10:15:10.057267724Z"
updated: "2026-11-23T19:00:21.466092Z"
tags:
  - crates
  - vcm
  - safety
attempts: 1
---

Move `common/ev_can.rs` and `crc8.rs` to `crates/ev-can`, without the bxcan dependency (plain data plus `embedded-can` conversions). Follow the wiki's `ev-can` page: torque request on `0x1D4` with torque big-endian in bytes 2-3, inverter DC voltage big-endian in bytes 0-1. Bytes the wiki does not document stay as the old code had them and are marked unverified. Host tests for every message and the CRC. Done when `just host ci` passes.