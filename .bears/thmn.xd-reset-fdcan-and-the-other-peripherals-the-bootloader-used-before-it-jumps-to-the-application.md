---
id: thmn.xd
title: Reset FDCAN and the other peripherals the bootloader used before it jumps to the application
status: open
priority: P2
created: "2026-11-23T19:59:32.443028817Z"
updated: "2026-11-23T19:59:32.443028817Z"
tags:
  - bootloader
---

Found in the review of the bootloader for thmn.am.

`state_application` (`ecus/bootloader/src/main.rs`) and `Can::reset` (`ecus/bootloader/src/can.rs`) carry TODOs: FDCAN has its clock gated but is not reset through `RCC_APB1RSTR1`. The CRC unit, which thmn.am adds, keeps its clock and configuration, and SysTick keeps its configuration. The application inherits that state. embassy's init probably copes, but nothing tests it.

## Acceptance
- Before the jump, the bootloader puts FDCAN, GPIOB, the CRC unit and SysTick back in their reset state through the RCC reset registers, and leaves the clock tree alone, as the wiki documents.
- `just bootloader ci` passes and the size change is reported.
- Bench check (manual): `can-echo` still starts and echoes after an update over CAN.