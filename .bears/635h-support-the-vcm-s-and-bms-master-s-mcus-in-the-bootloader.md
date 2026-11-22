---
id: 635h
title: Support the VCM's and BMS master's MCUs in the bootloader
status: open
priority: P2
created: "2026-11-22T10:20:58.818275492Z"
updated: "2026-11-22T10:20:58.818275492Z"
tags:
  - bootloader
  - vcm
  - bms
---

The bootloader builds for the STM32G431 and a 512 KiB STM32G491. The ECUs need:
- STM32F405RG (VCM main): F4 flash has sectors of 16/64/128 KiB, not pages, so `FlashLayout` and the erase code need a sector model; bxCAN, not FDCAN.
- STM32F091CC (VCM monitor): thumbv6m, 2 KiB pages, bxCAN.
- STM32G491CC (BMS master): 256 KiB, so the `stm32g491rx` layout's 512 KiB is wrong for it.

Each family is a feature and a module, as `ecus/bootloader/AGENTS.md` describes. Once one is supported, relink that ECU's application at the application offset instead of using embassy's `memory-x`.