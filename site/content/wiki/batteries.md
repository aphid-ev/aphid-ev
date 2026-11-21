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
manual service disconnect (MSD), and the precharge contactor and resistor.
Mounting the MSD and precharge hardware with the front pack keeps the
service-isolation point and precharge circuit in the same assembly.

The MSD contains the traction pack's 250 A main fuse.

# Rear battery

The rear battery contains positive and negative main contactors but no MSD
or precharge hardware. Its enclosure therefore contains only the six cell
modules, BMS electronics, main contactors, and their associated
connections.

# Wiring

![Schematic of the front battery: six cell modules, main contactors, MSD and precharge](/static/generated/wiring/front_battery.svg#inline "Front battery")

![Schematic of the rear battery: six cell modules and main contactors](/static/generated/wiring/rear_battery.svg#inline "Rear battery")

![Schematic of one 8s2p cell module and its monitoring connections](/static/generated/wiring/cell_module.svg#inline "Cell module")

# Current shunt

For current measurement on the pack:

- Shunt resistor: **BAS-M-R0001-R-5.0**
- Isolated ADC: **AMC3306M05**
