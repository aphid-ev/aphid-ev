---
id: aut3
title: Place CAN/USB timestamps correctly across quiet gaps of 71 minutes or more
status: proposed
priority: P3
created: "2026-11-24T19:18:40.268497147Z"
updated: "2026-11-24T19:18:40.307290274Z"
tags:
  - can-usb
---

`TimestampUnwrapper` counts at most one wrap per event, so two events more than 2^32 µs (71.6 min) apart come out whole periods too close. The host library could use the computer's monotonic clock at packet arrival to settle how many periods passed. Found in the epic `ayys` fixes. Checks: `just host ci`.