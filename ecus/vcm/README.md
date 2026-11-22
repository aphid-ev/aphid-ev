# VCM

Vehicle control module: the car's main controller. It has two MCUs, a main
STM32F405 for vehicle control and an STM32F091 monitor that supervises it.

- `hardware/` — KiCad project for the board, `aphid-vehicle-controller`.
- `firmware/main/` — embassy firmware for the main MCU.
- `firmware/monitor/` — embassy firmware for the monitor MCU.

Each firmware is a standalone cargo workspace. Both are starting points that
log a heartbeat and drive no pins, so they run on any board revision; no
revision has been tagged yet. Their logic lives in
[`crates/vcm-core`](../../crates/vcm-core/) and
[`crates/ev-can`](../../crates/ev-can/).

Recipes: `just vcm build | check | erc | drc | export | ci`, and
`just vcm flash main` or `just vcm flash monitor`.
