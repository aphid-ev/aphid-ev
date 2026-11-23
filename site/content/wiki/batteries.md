---
title: Batteries
category: Electronics
tags: [battery, electronics]
created: 2024-07-01
---

> [!NOTE]
> This page is just a draft — more schematics and diagrams coming.

The battery is split between front and rear enclosures. Each enclosure
contains six Renault Zoe 8s2p cell modules connected in series. The two
48s2p sections are connected in series to form a 96s2p traction pack.

Since the VW Beetle is quite small, the battery is divided into two parts:
a rear battery between the back seat and the rear firewall, and a front
battery in the frunk where the gas tank normally sits.

The pack ties into the rest of the [[HV system]] via the [[contactors]].
Each section is supervised by its own [[bms|BMS]].

# Front battery

The front battery contains positive and negative main contactors, the
manual service disconnect (MSD), the precharge contactor and resistor, and
the current shunt.
Mounting the MSD and precharge hardware with the front pack keeps the
service-isolation point and precharge circuit in the same assembly.

The MSD contains the traction pack's 250 A main fuse. It sits on the
pack's negative leg, between the negative contactor and the pack terminal,
and the [[safety#interlock|HV interlock loop]] passes through it. The
candidate part is described under
[[HV system#battery-string-and-fusing|HV system]].

# Rear battery

The rear battery contains positive and negative main contactors but no MSD
or precharge hardware. Its enclosure therefore contains only the six cell
modules, BMS electronics, main contactors, and their associated
connections.

# Wiring

![Schematic of the front battery: six cell modules, the main and precharge contactors with their coil wiring, the current shunt, the MSD and the BMS controller](/static/generated/wiring/front_battery.svg#inline "Front battery")

![Schematic of the rear battery: six cell modules, the main contactors with their coil wiring and the BMS controller](/static/generated/wiring/rear_battery.svg#inline "Rear battery")

![Schematic of one 8s2p cell module and its monitoring connections](/static/generated/wiring/cell_module.svg#inline "Cell module")

# Current shunt

One shunt measures the current of the whole string. It sits in the front
battery on the negative leg, between the last cell module and the negative
contactor, with its sense pair on the front [[bms|BMS]].

- Shunt resistor: **BAS-M-R0001-R-5.0**
- Isolated ADC: **AMC3306M05**
