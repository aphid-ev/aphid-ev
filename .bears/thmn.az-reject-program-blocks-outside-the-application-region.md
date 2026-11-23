---
id: thmn.az
title: Reject Program blocks outside the application region
status: done
priority: P1
created: "2026-11-23T19:00:57.488056Z"
updated: "2026-11-23T20:12:55.007772252Z"
tags:
  - bootloader
  - crates
  - safety
attempts: 1
---

Found in the review of qqxt.6x/qqxt.da; carried over unchanged from the original bootloader.

`MemoryRegion::absolute_address` (`ecus/bootloader/src/flash.rs:75`) adds the Program offset to the region start without checking it. In release builds this wraps: offset `0xFFFF_E000` (aligned, so the alignment check passes) gives `0x0800_0000`, which is the bootloader's own vector table. An all-zero block with a valid CRC can then overwrite the stack pointer and reset vector, and the ECU can only be recovered with a probe. Larger offsets reach OTP (`0x1FFF_7000`, where writes are permanent) or unmapped space. Nothing in `crates/boot-protocol` checks `offset + len` against the region either. This breaks the `ecus/bootloader/AGENTS.md` rule that the bootloader never writes its own region.

Must be fixed before a bootloader is flashed onto any board.

## Acceptance
- `boot-protocol` validates `offset + len` against the application region size with checked arithmetic, and answers out-of-range blocks with an existing error code (Overflow or Alignment). It only rejects invalid input, so the wire protocol stays compatible.
- The bootloader uses that validation before it erases or programs anything.
- Host tests cover the wrap-around offset, an offset just past the end, a block that straddles the end, and the last valid block.
- `site/content/wiki/bootloader.md` documents the check; until it lands, the limitations list there names the unchecked range.
- `just host ci` and `just bootloader ci` pass.