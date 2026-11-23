---
id: thmn.t2
title: Wait for the HSE crystal in the bootloader and fix the bootloader wiki
status: open
priority: P2
created: "2026-11-23T19:00:57.506752Z"
updated: "2026-11-23T19:00:57.506752Z"
tags:
  - bootloader
  - site
---

Found in the review of qqxt.da and qqxt.b3.

- `ecus/bootloader/src/main.rs:113` sets `hseon` and then waits for `hsirdy` rather than `hserdy`. The PLL lock wait probably hides it, but the bootloader can start the PLL before the crystal is running.
- `site/content/wiki/bootloader.md:102` says the node answers every command. Start application gets no response because the node jumps straight to the application. The original BOOT_PROTOCOL.md had the same error.

## Acceptance
- The clock setup waits for `hserdy` after enabling the HSE.
- The wiki states which commands get no response.
- `just bootloader ci` and `just site build` pass (or `site build` fails only for reasons unrelated to these pages).