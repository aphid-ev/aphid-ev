---
id: 2uxb
title: Exit the can-usb CLI with a message instead of a panic
status: open
priority: P3
created: "2026-11-24T19:18:30.466419731Z"
updated: "2026-11-24T19:18:30.466419731Z"
tags:
  - can-usb
---

The `can-usb` CLI's `main` ends errors with `expect("Error")`. Print the error and exit with a non-zero code instead (for example `DeviceNotFound` or `ProtocolMismatch` should say what to do). Found while implementing `ayys.da`. Checks: `just host ci`.