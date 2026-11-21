# CAN/USB interface

USB adapter for talking to the vehicle's CAN bus from a computer.

- `hardware/` — KiCad project for the board.
- `firmware/` — embassy firmware, a standalone cargo workspace.
- `host/` — host-side USB library, a member of the host workspace. Used by
  the manager app.

Recipes: `just can-usb build | check | flash | erc | drc | export | ci`. The
host library is built and tested by `just host ci`.
