# ECU guidelines

Applies to everything under `ecus/`. The bootloader has additional rules in
`bootloader/AGENTS.md`.

## Firmware

- Each `firmware/` is its own cargo workspace with its own `Cargo.lock`,
  `.cargo/config.toml` (target, runner) and `memory.x`. Run cargo from inside
  it, or use `just <ecu> build | check`. Do not add firmware crates to the
  root workspace.
- Application firmware is `no_std` Rust on embassy.
- Keep the binary thin: peripheral setup, task spawning and glue. Behaviour
  (state machines, limits, protocol handling) goes in a `crates/` library with
  host tests. If logic is hard to test, move it out of the firmware crate
  first.
- CAN messages, parameters and the bootloader protocol come from the shared
  crates. Never hard-code an ID or layout in a firmware.
- Shared crates are path dependencies (`../../../crates/<name>`). Keep embassy
  and HAL versions aligned across the firmwares when bumping one.
- No `unwrap`/`expect`/panicking indexing on paths that run while the vehicle
  is driving or charging. Handle the error and enter a defined fault state.
- A firmware must build with `just <ecu> build` before a change is done. There
  is no hardware in CI, so say what was and was not verified on a real board.
- Do not run `flash` recipes or probe-rs unless asked to.

## Hardware

- KiCad 10 projects. Treat `.kicad_sch` and `.kicad_pcb` as read-only: read
  them to answer questions, but layout and schematic changes are made in
  KiCad by the user. Do not hand-edit them.
- Boards are 4-layer. Each project's `<name>.kicad_dru` is a symlink to a
  shared rule file in `ecus/`: `pcb-4layer.kicad_dru` by default, which any
  mainstream prototype fab can make at its standard price, or
  `jlcpcb-4layer.kicad_dru` for a board that needs JLCPCB's tighter limits.
  Edit the shared file, not the rules in Board Setup.
- `just <ecu> erc` and `just <ecu> drc` run the rule checks;
  `just <ecu> export` writes schematics to `site/static/generated/hardware/`.
- Fabrication outputs and exports are not committed.
- The pin assignment in the schematic is the reference. When firmware pin
  mappings and the schematic disagree, report it; do not pick one silently.
- A firmware must state which board revisions it supports. Board revisions
  are tagged `<component>-hw-revX`.
