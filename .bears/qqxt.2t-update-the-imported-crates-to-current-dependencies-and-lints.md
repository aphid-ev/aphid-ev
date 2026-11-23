---
id: qqxt.2t
title: Update the imported crates to current dependencies and lints
status: done
priority: P2
created: "2026-11-22T09:10:12.438868801Z"
updated: "2026-11-23T19:00:21.433045Z"
tags:
  - can-usb
  - bootloader
  - crates
depends_on:
  - qqxt.79
  - qqxt.xb
attempts: 1
---

The imported code was last touched months ago. Bump every dependency to its latest release (embassy, stm32g4 PAC, fdcan, nusb, tokio, clap, …), fix what breaks, and clean up anything the latest clippy flags. Keep embassy versions aligned across firmwares. Done when every recipe in the epic passes on the latest stable toolchain.