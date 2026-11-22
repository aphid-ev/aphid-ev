---
id: v438
title: Re-enable ERC and DRC in CI for the VCM and BMS
status: open
priority: P2
created: "2026-11-22T22:44:19.422370116Z"
updated: "2026-11-22T22:44:19.422370116Z"
tags:
  - vcm
  - bms
  - ci
---

ERC and DRC are switched off in CI for the VCM and BMS boards while the overall system design is being settled. The boards were imported with open ERC/DRC errors, and fixing them before the design is final is wasted effort.

Turned off in:
- `.github/workflows/vcm.yml` and `.github/workflows/bms.yml`: `hardware: false`
- `ecus/vcm/justfile` and `ecus/bms/justfile`: `ci` runs only `ci-firmware`

## Done when
- `just vcm ci-hardware` and `just bms ci-hardware` pass (fix the boards, and consider moving them to the shared 4-layer design rules)
- `hardware: false` and the comments that point at this task are removed from both workflows
- `ci` in both unit justfiles is back to `ci-firmware ci-hardware`

Blocked on the system design being final, not on another task.