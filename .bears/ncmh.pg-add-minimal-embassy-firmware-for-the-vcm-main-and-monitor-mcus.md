---
id: ncmh.pg
title: Add minimal embassy firmware for the VCM main and monitor MCUs
status: review
priority: P2
created: "2026-11-22T10:15:18.565983342Z"
updated: "2026-11-22T10:21:06.041403770Z"
tags:
  - vcm
depends_on:
  - ncmh.z9
  - ncmh.yf
attempts: 1
---

Two standalone workspaces, `ecus/vcm/firmware/main` (STM32F405RG, thumbv7em-none-eabihf) and `ecus/vcm/firmware/monitor` (STM32F091CC, thumbv6m-none-eabi). Each initialises embassy and logs a heartbeat; no pins until they are taken from the schematic. The VCM justfile builds and checks both, and the workflow caches both. Done when `just vcm ci-firmware` passes.