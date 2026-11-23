---
title: Mitsubishi Outlander OBC/DC-DC
category: Components
tags: [charging, dcdc, electrical]
created: 2026-11-21
---

The original plan used a [[pdm|Nissan Leaf power delivery module]], but it
does not fit the available space. Charging and low-voltage conversion
therefore use a more easily packaged combined on-board charger (OBC) and
DC-DC converter from a Mitsubishi Outlander PHEV. The OBC charges the
traction battery from single-phase AC. The DC-DC converter supplies the
12 V system and maintains the auxiliary battery. The VCM controls the unit
over CAN1 and two discrete signals.

Facts about the unit from the sources listed under [[#sources]]:

| Property | Value |
| --- | --- |
| Charger output | 3.3 kW, at most 12 A DC |
| AC input | Single phase, about 14.5 A at full power |
| DC-DC output | 1.8 kW at about 14.4 V, about 125 A |
| HV bus range | 200 V to 400 V |
| Size and weight | 370 x 270 x 150 mm, about 12 kg |
| Cooling | Liquid |

The traction pack stays below 400 V, which is where the unit stops
working. Requesting more than 12 A makes the charger refuse to start.

# Connectors {#connectors}

The pin numbers come from the Mitsubishi circuit diagrams, which name the
connectors H-108, H-101, D-205 and D-208.

## HV connector

Two pins: pin 1 is negative and pin 2 is positive. The connector has no
interlock contacts, so the unit is excluded from the
[[safety#interlock|HV interlock loop]].

The unit has its own fused branch from the HV junction box, described
under [[HV system#battery-string-and-fusing|HV system]]. The
branch has a 30 A fuse, the rating Mitsubishi uses, and 6 mm² cable. The
charger draws at most 12 A and the DC-DC about 9 A at the lowest bus
voltage, so the two together stay near 21 A.

The unit has no contactor of its own, so its input capacitors charge
together with the inverter's during [[contactors#precharge|precharge]].
The VCM keeps the DC-DC disabled until precharge is complete. A running
DC-DC would draw its load through the precharge resistor and the bus would
never reach pack voltage.

## AC connector

A five-way housing, Yazaki 7283-7350-30, with three cavities used.

| Pin | Signal |
| --- | --- |
| 1 | Neutral |
| 2 | Line |
| 4 | Protective earth |

Mitsubishi screens the line and neutral conductors and earths the screens
to the body.

## 12 V output

A single M8 stud carries the positive output. The return is the aluminium
case, so the case needs a ground strap as heavy as the output cable. The
output cable is 35 mm² behind a 150 A MIDI fuse in the primary
distribution box, sized for the full 125 A. See
[[LV system#primary-distribution]].

## Signal connector

A 13-way connector on a short tail. The mating half is Sumitomo 6189-1092.

| Pin | Signal | Connects to |
| --- | --- | --- |
| 4 | DC SW, DC-DC enable | VCM output |
| 5 | CHIN | Not used |
| 6 | CAN H | CAN1 |
| 7 | Sense | Always-on 12 V, 7.5 A fuse |
| 8 | IGCT, main 12 V supply | EV relay |
| 9 | Control pilot | Charge inlet CP |
| 10 | Ground | VCM ground |
| 12 | CHOT | Not used |
| 13 | CAN L | CAN1 |

Pins 1, 2, 3 and 11 are empty. CHIN and CHOT are a serial link to
Mitsubishi's remote-control unit.

A note on the openinverter wiki says the numbers moulded on the connector
are reversed. The Mitsubishi diagram agrees with the table above. Check
the wire colours on the unit's tail before crimping: grey DC SW, black
CAN H, green sense, yellow IGCT, white control pilot, black ground and red
CAN L.

# Supply and wake-up {#supply}

The sense input is always live. IGCT is fed from the EV relay in the
[[LV system#rear-distribution|rear EV distribution box]], which the VCM
pulls in for driving and for charging. The unit only charges with IGCT
powered. The control pilot alone does not wake it, and builders on the
openinverter forum report faults when IGCT is powered permanently.

The EV relay also feeds the inverter's ignition input, so the inverter is
awake while the car charges.

The current the sense input draws while the car sleeps is not documented.
Measure it on the bench.

# CAN interface

The unit is an unterminated intermediate node on CAN1, between the VCM and
the inverter, at 500 kbit/s. It has no termination resistor. See
[[EV-CAN#vehicle-buses]].

# DC-DC {#dcdc}

The Mitsubishi DC-DC converter maintains the auxiliary 12 V battery and
supplies the 12 V system whenever the HV system is active, in a similar
fashion to the alternator of a car with a combustion engine. It needs
12 V on sense and IGCT, then 12 V on DC SW. It runs without any CAN
message.

# Cooling {#cooling}

The unit is liquid cooled. The coolant runs through the radiator, the OBC,
the inverter and the motor in series, and the pump runs while the car
charges. The OBC sits first after the radiator, where the coolant is
coldest. The pump relay shares the EV relay's control line, so the pump has
power whenever IGCT has.

The port size and the flow the unit needs are not documented in the
sources. Check that its ports do not restrict the flow to the inverter and
motor.

# Charge inlet {#inlet}

The inlet is a BESEN BS-EVSC004 Type 2 inlet, rated 32 A, ordered with
16 A single-phase tails of 2.5 mm². L1, neutral and protective earth
connect to the OBC. L2 and L3 are unused.

- The control pilot (CP) runs straight to pin 9 of the OBC signal
  connector. The OBC holds the pilot resistors, signals the charging point
  that it is ready, and limits its current to what the charging point
  advertises.
- The proximity pilot (PP) connects to the VCM. The OBC has no PP input.
  PP tells the VCM that a cable is plugged in, wakes it from sleep and
  blocks driving. The resistance from PP to earth also gives the cable's
  current rating.

## Status LEDs

The inlet has a red and a green lens with room for an LED behind each, but
no LEDs. Two 12 V indicator LEDs are fitted, each on its own VCM output,
with a shared return to the VCM.

| LED | Meaning |
| --- | --- |
| Green, pulsing | Charging |
| Green, steady | Charged |
| Red | Fault |

## Lock {#lock}

A Type 2 plug has no latch, so only friction holds it in the inlet. The
BESEN BS-EL0003 lock drives a bolt into the plug. The lock is designed in
but optional: the car charges without it.

| Property | Value |
| --- | --- |
| Supply | 12 V DC, 10.5 V to 13.5 V |
| Current | 650 mA maximum |
| Drive | +12 V on the red lead locks, reversed polarity unlocks |
| Pulse | 0.2 s to 1 s, never longer than 3 s |
| Feedback | Switch between the white and blue leads, closed while locked |
| Tail | 500 mm, four wires of 24 AWG |

The bolt stays in place without power. The VCM drives the motor from an
H-bridge and reads the feedback switch on one input. It unlocks the inlet
when the charge is finished, before it goes to sleep, so the cable comes
out without the key. A stop button beside the inlet ends a charge early and
unlocks. The lock's manual release covers a flat 12 V battery.

The charge sequence is described under
[[vcm#operating-states|operating states]].

## Charge wiring

![Schematic of the charging control signals: CP to the OBC, and PP, the DC/DC enable, the lock, the status LEDs and the stop button to the VCM](/static/generated/wiring/charging_control.svg#inline "Charging control")

![Harness drawing of the leads between the VCM charge connector, the inlet, the lock, the stop button and the OBC/DC-DC](/static/generated/wiring/charge_signal_harness.svg#inline "Charge signal harness")

![Harness drawing of the AC lead from the charge inlet to the OBC](/static/generated/wiring/ac_charge_harness.svg#inline "AC charge harness")

# Sources {#sources}

- Mitsubishi Outlander PHEV service manual, circuit diagrams
  HKAE0E05AC00ENG, HKAE0E05BC00ENG, HKAE0E05CC00ENG and HKAE0E06AC00ENG:
  connector pin numbers, fuses and the pilot wiring.
- [openinverter wiki, Mitsubishi Outlander DCDC OBC](https://openinverter.org/wiki/Mitsubishi_Outlander_DCDC_OBC):
  ratings, connector part numbers, wire colours and CAN messages. It links
  to the circuit diagrams.
- [openinverter forum, Outlander charger thread](https://openinverter.org/forum/viewtopic.php?t=628):
  IGCT behaviour, voltage limits, cooling and case grounding, as reported
  by builders.
- BESEN datasheet for the BS-EL0003 lock and the evshop.eu listings for the
  [inlet](https://evshop.eu/en/plugs-sockets/178-type-2-ev-socket-3-phase-32a.html)
  and the
  [lock](https://evshop.eu/en/plugs-sockets/176-electronic-lock.html).
