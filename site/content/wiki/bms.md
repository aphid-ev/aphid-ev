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
distribution box in the [[LV system#primary-distribution|LV system]]. A
controller sleeps with only the wake receiver of its CAN transceiver
alive. Traffic on CAN2 wakes it: the transceiver switches the board's
supply on, and the controller then keeps itself alive from the always-on
feed and decides when to sleep again. The VCM can therefore reach a
BMS without the contactor coil supply being on, and so can the
[[can-usb|CAN/USB interface]] during service.

The transceiver needs a sleep mode with a wake receiver on the bus and an
output that switches the board's regulator, such as the NXP TJA1043 or the
TI TCAN1043. The part is not chosen. Its sleep current, tens of microamps
for these parts, is the drain on the 12 V battery while the car is off.

Safe power from the VCM is a monitoring input on the board. It tells the
BMS whether the contactor coil supply is present.

# I/O

The drawings show every port of each BMS controller in the wiring model and
what it connects to inside its pack, or where it leaves the pack.

![Schematic of the front BMS controller with every port and a label naming what it connects to](/static/generated/wiring/front_bms_i_o.svg#inline "Front BMS I/O")

![Schematic of the rear BMS controller with every port and a label naming what it connects to](/static/generated/wiring/rear_bms_i_o.svg#inline "Rear BMS I/O")

Safe power arrives on the pack's signal connector with the other
low-voltage signals. Inside the pack it feeds the contactor coils and the
BMS monitoring input.

Each board has three high-voltage inputs on a connector of their own, apart
from the low-voltage connector, so the board can keep them isolated:

- A sense pair for the current shunt.
- A voltage sense pair on the cell stack, before the contactors.
- A voltage sense pair on the pack's output terminals, after the
  contactors.

The BMS reports both voltages on CAN2. They agree once precharge is
complete, and the VCM takes that decision from them; see
[[contactors#precharge]]. A voltage on the output terminals while the
contactors are open shows a welded contactor.

Both packs wire the two voltage sense pairs. The current shunt is in the
front pack, so only the front BMS uses its shunt input. The packs are in
series, so that one shunt measures the current of the whole string, and the
front BMS reports it on CAN2. The enclosure needs two separate connectors
for the high-voltage inputs.

The precharge contactor is in the front pack, on the front BMS's third
output. Every board has the third output, and the rear BMS leaves it
unused.

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
