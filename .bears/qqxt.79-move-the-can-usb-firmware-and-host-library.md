---
id: qqxt.79
title: Move the CAN/USB firmware and host library
status: done
priority: P2
created: "2026-11-22T09:10:01.396537905Z"
updated: "2026-11-23T19:00:21.374349Z"
tags:
  - can-usb
depends_on:
  - qqxt.3u
attempts: 1
---

`firmware` → `ecus/can-usb/firmware` (own workspace and lock file), `app` → `ecus/can-usb/host` as a library with the `listen`/`ping`/`throughput` CLI as a binary, plus the udev rule. Both use `crates/can-usb-protocol`. Done when `just can-usb ci-firmware` and `just host ci` pass.