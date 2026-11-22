---
id: ncmh.yf
title: Add the VCM logic crate
status: review
priority: P2
created: "2026-11-22T10:15:10.075791293Z"
updated: "2026-11-22T10:19:45.407446621Z"
tags:
  - crates
  - vcm
  - safety
attempts: 1
---

Move throttle, timeout, input, direction and the main/monitor messages from `common`, and the throttle and torque monitors from `monitor-app`, to `crates/vcm-core` with host tests. Fix the inverted throttle validation in `input.rs` and the torque monitor that never resets its frame timeout. Drop the `todo!()` serial stub and the empty main-app monitor. No embassy-time dependency. Done when `just host ci` passes.