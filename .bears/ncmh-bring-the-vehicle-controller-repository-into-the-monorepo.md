---
id: ncmh
title: Bring the vehicle-controller repository into the monorepo
type: epic
status: open
priority: P1
created: "2026-11-22T10:15:03.095799301Z"
updated: "2026-11-22T10:15:03.095799301Z"
tags:
  - vcm
  - bms
  - crates
  - site
---

Move `~/dev/vehicle-controller` (github.com/aphid-ev/vehicle-controller at 3feb67b) into the monorepo as a fresh copy, stacked on epic qqxt. The direction controller (board and `direction-app`) is dropped: the shifter is wired to the VCM.

- `common` → `crates/ev-can` (Nissan EV-CAN messages, following the wiki: torque request `0x1D4`, big-endian) and `crates/vcm-core` (throttle, timeout, direction, main/monitor messages, monitor checks), with tests and the bugs fixed.
- `main-app`/`monitor-app` prototypes are replaced by minimal embassy firmware for the STM32F405 and STM32F091 in `ecus/vcm/firmware/{main,monitor}`.
- VCM board → `ecus/vcm/hardware`, BMS master → `ecus/bms/hardware/master`, the shared KiCad library → `ecus/libraries`. ERC/DRC are enforced, so the hardware jobs fail until the designs are fixed in KiCad.
- Wiki pages point at the monorepo.