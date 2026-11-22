---
id: at8z
title: Fix the VCM and BMS master ERC and DRC errors in KiCad 10
status: open
priority: P2
created: "2026-11-22T10:21:54.594669668Z"
updated: "2026-11-22T10:21:54.594669668Z"
tags:
  - manual
  - vcm
  - bms
  - hardware
---

Both boards came from vehicle-controller as KiCad 7 files (format 20231120) with unfinished designs, so `ci-hardware` fails for both units. Under kicad-cli 10.0.8:
- VCM (`ecus/vcm/hardware`): ERC 189 errors (138 pin_not_connected, 16 power_pin_not_driven, 16 label_dangling, 14 pin_not_driven, 3 hier_label_mismatch, 1 pin_to_pin). DRC 42 unconnected items and 1 invalid outline.
- BMS master (`ecus/bms/hardware/master`): ERC clean. DRC 45 unconnected items, 34 clearance, 34 hole clearance.

Open each project in KiCad 10 (which upgrades the format), fix or waive the violations, and save. Done when `just vcm ci-hardware` and `just bms ci-hardware` pass.