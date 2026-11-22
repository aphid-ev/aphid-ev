---
id: qqxt.3u
title: Add the CAN/USB wire protocol crate
status: review
priority: P2
created: "2026-11-22T09:09:55.343059385Z"
updated: "2026-11-22T09:11:29.534647702Z"
tags:
  - crates
  - can-usb
depends_on:
  - u4mx
attempts: 1
---

Move `common` to `crates/can-usb-protocol`. Replace the `embassy` feature's conversions with ones over `embedded-can` traits, so the crate has no HAL dependency. Keep the wire encoding and its pinned-bytes tests unchanged. Done when `just host ci` passes.