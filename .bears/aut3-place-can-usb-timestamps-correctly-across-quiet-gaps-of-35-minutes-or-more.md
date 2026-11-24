---
id: aut3
title: Place CAN/USB timestamps correctly across quiet gaps of 35 minutes or more
status: proposed
priority: P3
created: "2026-11-24T19:18:40.268497147Z"
updated: "2026-11-24T19:22:47.589114636Z"
tags:
  - can-usb
---

`TimestampUnwrapper` counts at most one wrap per event, so two events more than 2^31 µs (35.8 min) apart come out wrong (the unwrapper reads a larger step as a step back). The host library could use the computer's monotonic clock at packet arrival to settle how many periods passed. Found in the epic `ayys` fixes. Checks: `just host ci`.