---
title: Wiring drawings
category: Wiring
tags: [wiring, electronics]
created: 2026-11-23
---

The drawings of the car's electrical system are generated from one wiring
model, kept in [`wiring/`](https://github.com/aphid-ev/aphid-ev/tree/main/wiring)
in the repository. The model covers the EV systems and the circuits the
conversion changes. The rest of the Beetle's original electrical system is
not modelled.

The pages about each system show its schematic. The harness and connector
drawings needed to build the looms are collected here:

- [[Cab loom]]: the branches to the key, the accelerator sensor, the brake
  switches and the gear lever.
- [[Rear loom]]: the leads around the VCM in the engine compartment, the
  battery signal loom through the tunnel and the charge inlet leads.
- [[HV cables]]: the traction cables, the motor cables and the AC charge
  lead.
- [[pinouts]]: connector faces with a table of what lands on each pin.

# Schematics

| System | Page |
| --- | --- |
| HV bus | [[HV system]] |
| Battery packs and cell modules | [[batteries]] |
| Interlock loop and safe power | [[safety]] |
| 12 V supply, fuse boxes, grounds and lights | [[LV system]] |
| CAN buses | [[EV-CAN]] |
| VCM signals and driver inputs | [[vcm]] |
| BMS signals | [[bms]] |
| Gear lever and parking brake switch | [[direction]] |
| Charging control | [[obc]] |
| Coolant pump and fan | [[waterpump]] |

# Reading a harness drawing

A harness drawing shows each connector as a table of its pins, with the
conductors between them. A box between two connectors is a cable, and its
header gives the cable type, the length and the number of conductors. Each
conductor is labelled with its signal, its cross-section and its colour.

Colours use the IEC 60757 codes. A two-tone wire is written base colour
first, then the tracer. CAN pairs are white with a blue tracer for CAN high
and white with a red tracer for CAN low.

An inline connector is a mated pair where one loom hands over to another.
Each loom's drawing shows the half that is crimped onto that loom, marked F
for the female half and M for the male half.

A harness drawing only shows a conductor when both of its ends are on a
connector in the model. The two fuse boxes, the 12 V battery and the
emergency stop have no connectors in the model yet, so the leads to them are
missing from the harness drawings. The schematics show them.
