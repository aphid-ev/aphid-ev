---
id: bmd5
title: Fix stale VCM flashing and bootloader notes in the docs
status: open
priority: P3
created: "2026-11-23T19:00:57.584301Z"
updated: "2026-11-23T19:00:57.584301Z"
tags:
  - site
  - vcm
---

Found in the review of ncmh.pg and ncmh.fv.
- `ecus/README.md` shows `just vcm flash` with no argument; the recipe needs `main` or `monitor` (`ecus/vcm/justfile:26`).
- `site/content/wiki/vcm.md:236-238` says the CAN bootloader doesn't support the STM32F405 or STM32F091 yet. Since the move to the G491/G431 that sentence is stale.
- `site/content/wiki/ev-can.md:102-103` says the `???` bytes "are encoded as the prototype had them"; for 0x1DA they are decoded.

## Acceptance
- The three passages are corrected. `just site build` passes, or fails only for unrelated reasons.