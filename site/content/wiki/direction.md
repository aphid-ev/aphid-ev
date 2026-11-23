---
title: Direction controller
category: Electronics
tags: [software, hardware, electronics]
created: 2024-06-03
---

The driver selects the direction with the Beetle's original gear lever,
mounted in a new box and sprung to centre. Its switches connect directly to
the [[vcm#io|VCM]], which keeps the selected state.

# Shifting

The states are in the order R, N, D, as on a conventional automatic.
Pushing the lever forward moves one step toward R, and pulling it back
moves one step toward D. From D, one push forward gives N and a second
gives R.

- A step into N is always accepted, at any speed and without the brake.
- A step into D or R is accepted only while the car is stopped with the
  brake applied.
- The lever has to return to centre before the next step counts.
- The VCM starts in N at key on.

The lever does not show the selected state. A later display ECU will show
it; until then the driver keeps track.

# Switches

Each direction has two microswitches wired as a complementary pair. Switch
1 is normally open and switch 2 is normally closed. Both take 12 V from the
switch fuse in the
[[LV system#primary-distribution|primary distribution box]].

| Switch 1 | Switch 2 | Meaning |
| --- | --- | --- |
| Low | High | Lever at rest |
| High | Low | Lever moved that way |
| Low | Low | Fault: feed lost, wire open or short to ground |
| High | High | Fault: short to 12 V or between the two wires |

The reaction to a fault is not specified yet.

A single parking brake switch shares the feed and the loom. It is a warning
input and has no second switch.

![Schematic of the switch fuse feeding the gear lever and the parking brake switch, with their lines on the VCM gear connector](/static/generated/wiring/direction_selector.svg#inline "Direction selector")

![Harness drawing of the gear loom from the inline connector to the gear lever and the parking brake switch](/static/generated/wiring/gear_loom.svg#inline "Gear loom")

> [!WARNING]
> The board below predates the current electrical architecture. The
> CAN-based controller is kept for reference.

# Direction controller board

The direction controller is a small PCB used as a driver interface to
select drive direction. It does not contain any logic, just enough to
make 3 buttons and 3 LEDs available to the driver:

- Forward
- Neutral
- Reverse

The button state is communicated to the [[vcm]] which controls
the state of the LEDs.

## Hardware

The hardware is created using [KiCad](https://www.kicad.org) and can be
found in the
[vehicle-controller](https://github.com/aphid-ev/vehicle-controller/tree/main/hardware/aphid-direction-controller)
GitHub repository.

### PCB

*TBD.*

### Connectors

The direction controller has four 4-pin JST-PH connectors:

- Connector housing: **JST PHR-4**
- Crimp contact: **JST SPH-002T-P0.5S**

#### Main connector

| Pin | Function |
| --- | --- |
| 1 | CAN High |
| 2 | CAN Low |
| 3 | +12 V (Ignition) |
| 4 | GND |

#### Button connectors (×3)

The button connector has the same pinout for forward, neutral and reverse:

| Pin | Function |
| --- | --- |
| 1 | LED Anode |
| 2 | LED Cathode |
| 3 | Button |
| 4 | Button |

The current-limiting resistors on the PCB are sized to provide ~10 mA to
the following LED forward voltages. If different LEDs are used, the
resistors **R?**, **R?** and **R?** may need to be replaced.

- Forward: **2.8 V** *(blue)*
- Neutral: **3.5 V** *(white)*
- Reverse: **2.8 V** *(blue)*

## Firmware

The firmware for the STM32G431CB is written in Rust using
[Embassy](https://embassy.dev) and is very simple. It listens to a *TBD*
frame from *TBD* CAN containing the LED state, and responds with a frame
containing the state of the push buttons.

The firmware lives in the
[vehicle-controller](https://github.com/aphid-ev/vehicle-controller/tree/main/firmware/direction-app)
GitHub repository.
