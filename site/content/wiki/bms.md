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

# Packaging

The pack-level BMS controllers are enclosed ECUs. The cell-monitoring front
ends are an exception: each is a bare PCB mounted directly on its Renault
Zoe cell module rather than housed in a separate ECU enclosure.

The VCM, front BMS controller, and rear BMS controller use Cinch ModICE
enclosures with Molex MX-150 connectors. This packaging convention applies
to the enclosed DIY ECUs; it does not apply to the bare cell-monitoring
PCBs mounted on the battery modules.

# Current shunt

- Shunt resistor: **BAS-M-R0001-R-5.0**
- Isolated ADC: **AMC3306M05**
