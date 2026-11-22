# CAN/USB interface

USB adapter for talking to the vehicle's CAN bus from a computer. See
[the wiki page](https://bladlus.se/wiki/can-usb/) for how it works.

- `hardware/` — KiCad project for the board.
- `firmware/` — embassy firmware for the STM32G431CB, a standalone cargo
  workspace. No board revision has been tagged yet; the pins it uses are
  USB on PA11/PA12 and FDCAN1 on PB8/PB9, with a 16 MHz crystal.
- `host/` — host-side USB library, a member of the host workspace, used by
  the manager app. It also builds `can-usb`, a test CLI.
- `host/70-can-usb.rules` — udev rule for access without root on Linux.

Recipes: `just can-usb build | check | flash | erc | drc | export | ci`. The
host library is built and tested by `just host ci`.

```sh
cargo run -p can-usb-host -- listen      # print every frame on the bus
cargo run -p can-usb-host -- ping        # send a frame every 5 s, print the reply
cargo run -p can-usb-host -- throughput  # 1000 round trips as fast as possible
```
