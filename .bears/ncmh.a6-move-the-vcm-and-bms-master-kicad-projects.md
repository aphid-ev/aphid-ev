---
id: ncmh.a6
title: Move the VCM and BMS master KiCad projects
status: review
priority: P2
created: "2026-11-22T10:15:18.585240295Z"
updated: "2026-11-22T10:22:09.454571629Z"
tags:
  - vcm
  - bms
attempts: 1
---

Copy the VCM board to `ecus/vcm/hardware`, the BMS master to `ecus/bms/hardware/master` and the shared library to `ecus/libraries`, keeping the project file names (KiCad keys annotations to them). Repoint the library tables. ERC/DRC stay enforced: as of 3feb67b the VCM has 189 ERC and 43 DRC errors and the BMS master 113 DRC errors under KiCad 10. Done when `just vcm export` and `just bms export` work.