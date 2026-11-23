---
title: DIY BMS
category: Electronics
tags: [battery, electronics]
created: 2024-08-04
---

Each of the two [[batteries|battery sections]] has a DIY BMS. Analog
cell-monitoring front ends measure cell voltages and temperatures and
communicate with the local BMS controller over an isoSPI daisy chain.
Both BMS controllers switch the low side of their main contactor coils;
the front BMS also controls the precharge contactor. Each BMS reports pack
status over CAN2.

[[contactors#coil-control]] describes how the BMS and VCM share contactor
control, and [[EV-CAN#vehicle-buses]] describes the CAN2 topology.

Both BMS controllers have an always-on 12 V supply from the primary
distribution box in the [[LV system#primary-distribution|LV system]], so
they stay on while the car is off.

# I/O

The drawings show every port of each BMS controller in the wiring model and
what it connects to inside its pack, or where it leaves the pack.

![Schematic of the front BMS controller with every port and a label naming what it connects to](/static/generated/wiring/front_bms_i_o.svg#inline "Front BMS I/O")

![Schematic of the rear BMS controller with every port and a label naming what it connects to](/static/generated/wiring/rear_bms_i_o.svg#inline "Rear BMS I/O")

Each board has three high-voltage inputs on a connector of their own, apart
from the low-voltage connector, so the board can keep them isolated:

- A sense pair for the current shunt.
- A voltage sense pair on the cell stack, before the contactors.
- A voltage sense pair on the pack's output terminals, after the
  contactors.

The two voltages agree once precharge is complete, so the BMS can run
precharge without the inverter being online. A voltage on the output
terminals while the contactors are open shows a welded contactor.

Which pack uses which input is not decided, so the model leaves them
unconnected. The enclosure needs two separate connectors for this.

Which pack gets the precharge contactor is not decided either. It depends
on the space in each pack. This page and the
[[contactors|contactors page]] name the front pack, and the model draws it
in the rear pack on the rear BMS's third output. Every board has the third
output, so the choice does not affect the board.

# Packaging

The pack-level BMS controllers are enclosed ECUs. The cell-monitoring front
ends are an exception: each is a bare PCB mounted directly on its Renault
Zoe cell module rather than housed in a separate ECU enclosure.

The VCM, front BMS controller, and rear BMS controller use Cinch ModICE
enclosures with Molex MX-150 connectors. This packaging convention applies
to the enclosed DIY ECUs; it does not apply to the bare cell-monitoring
PCBs mounted on the battery modules.

# Microcontroller

Each BMS controller runs on an STM32G491CE with 512 KB of flash, the same
part as the [[vcm]] main MCU. The current schematic has the STM32G491CC,
the 256 KB version in the same package.

# Current shunt

- Shunt resistor: **BAS-M-R0001-R-5.0**
- Isolated ADC: **AMC3306M05**
