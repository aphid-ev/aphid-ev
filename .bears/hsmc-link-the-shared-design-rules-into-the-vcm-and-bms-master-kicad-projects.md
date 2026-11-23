---
id: hsmc
title: Link the shared design rules into the VCM and BMS master KiCad projects
status: open
priority: P2
created: "2026-11-23T19:00:57.563695Z"
updated: "2026-11-23T19:00:57.563695Z"
tags:
  - vcm
  - bms
  - hardware
---

Found in the review of ncmh.a6. `ecus/AGENTS.md` requires a `<name>.kicad_dru` symlink to `ecus/pcb-4layer.kicad_dru` in each KiCad project; can-usb has one, the VCM and BMS master don't. The BMS master has a `.kicad_pcb`, so its DRC currently runs without the shared rules.

This is a file-system change, not KiCad editing.

## Acceptance
- Relative symlinks named after each project's board file, as in `ecus/can-usb/hardware/`.
- `just vcm drc` and `just bms drc` pick up the shared rules where kicad-cli is available (they may still fail for the reasons in at8z).