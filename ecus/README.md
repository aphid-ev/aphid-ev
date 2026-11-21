# ECUs

One directory per unit, with its KiCad hardware and Rust firmware side by side.

| Directory | Unit | Hardware | Firmware |
| --- | --- | --- | --- |
| [`can-usb/`](can-usb/) | CAN/USB interface | one board | embassy, plus a host USB library in `host/` |
| [`vcm/`](vcm/) | Vehicle control module, the car's main controller | one board | embassy |
| [`bms/`](bms/) | Battery management system | master board and one slave board per battery module | embassy, master only |
| [`bootloader/`](bootloader/) | Bootloader shared by all ECUs | — | plain Rust, one build per MCU family |

Each firmware is a standalone cargo workspace: its target and probe-rs runner
are set in `firmware/.cargo/config.toml`, and it uses the shared crates in
[`crates/`](../crates/) by path.

`firmware.just` and `hardware.just` hold the recipes every unit shares; each
unit's `justfile` imports the ones it needs.

```sh
just vcm build     # release firmware
just vcm check     # fmt and clippy
just vcm flash     # flash and run through probe-rs
just vcm erc       # electrical rules check on every schematic
just vcm drc       # design rules check on every board
just vcm export    # schematic PDF and SVGs into site/static/generated/hardware/vcm
```
