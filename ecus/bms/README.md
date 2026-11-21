# BMS

Battery management system: one master and one slave board per battery module.

- `hardware/master/` — KiCad project for the master board.
- `hardware/slave/` — KiCad project for the slave board. It has no firmware.
- `firmware/` — embassy firmware for the master, a standalone cargo workspace.

Recipes: `just bms build | check | flash | erc | drc | export | ci`. The
hardware recipes cover both boards.
