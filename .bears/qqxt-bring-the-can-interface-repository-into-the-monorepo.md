---
id: qqxt
title: Bring the can-interface repository into the monorepo
type: epic
status: done
priority: P1
created: "2026-11-22T09:09:49.678727167Z"
updated: "2026-11-23T19:00:21.453212Z"
tags:
  - can-usb
  - bootloader
  - crates
  - site
---

Move `~/dev/can-interface` (github.com/aphid-ev/can-interface at 38ad43f) into the monorepo as a fresh copy without history:

- `common` → `crates/can-usb-protocol`, without the embassy-stm32 dependency.
- `firmware` → `ecus/can-usb/firmware`; `app` → `ecus/can-usb/host` (library plus test CLI).
- Bootloader protocol logic → `crates/boot-protocol` with host tests; `can_bootloader` → `ecus/bootloader`, keeping only flash, CAN and jump code.
- `can_echo` → `ecus/bootloader/can-echo`, the test application.
- `BOOT_PROTOCOL.md` and the README's design notes → wiki pages.
- `xtask` is replaced by just recipes.
- Then bring every dependency up to date and pass the latest clippy.