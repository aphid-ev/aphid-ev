---
title: Main contactors and precharge
category: Electronics
tags: [battery, electronics]
created: 2024-07-01
---

> [!NOTE]
> This page is just a draft — more schematics and diagrams coming.

The main contactors make it possible to keep the HV system de-energised
during all drive states. They are an important part of the
[[safety]] and the last resort to bring the system into a
safe state in case of a hazardous situation.

Together with the precharge circuit, they are also an important part of
[[vcm#operating-states|starting]] up the vehicle.

# Main contactors

Each of the [[batteries]] has a positive and a negative main contactor.
The positive and negative main contactors are
[Littelfuse DCNHE250MFA-B](https://www.littelfuse.com/products/relays-contactors-transformers/high-voltage-dc-contactor-relays/dcnhe250/dcnhe250mfa-b)
units.

## Welded-contactor detection

A welded contactor is detected from the [[bms|BMS]] voltage measurements.
Each BMS measures its pack's output terminals behind the two main
contactors, so a voltage there while both are commanded open means both
have welded.

That measurement spans both contactors of the pack. One welded contactor
shows nothing while the other is open. Finding it takes a check that closes
one contactor alone and looks for voltage on the output. The sequence for
that check is not specified yet.

The contactors also have mechanically linked normally-open auxiliary
contacts. They are not wired.

# Coil control

Contactors require authorization from both the VCM safety path and the
relevant BMS:

- The secondary safety processor in the [[vcm|VCM]] enables a high-side
  driver that supplies the battery contactor coils, as long as the
  [[safety#interlock|HV interlock loop]] is valid.
- Both [[bms|BMS]] controllers switch the low side of their main
  contactor coils. The front BMS also controls the precharge contactor.
- The same high-side supply is a monitoring input on each BMS, so the BMS
  sees when the VCM takes it away.

# Precharge

Precharge is managed by a Littelfuse `DCNSEV30-B` in series with a
`10 Ω 100 W` power resistor. These are connected in parallel to the
positive main contactor in the [[batteries#front-battery|front battery]].

Each [[bms|BMS]] measures two voltages in its pack: the cell stack between
the contactors, and the pack's output terminals behind them. It reports
both on CAN2. While the front pack precharges, its output voltage rises
toward its stack voltage, and the two agree once precharge is complete.

The VCM decides when precharge is complete from the BMS measurements. It
also compares them with the HV bus voltage the [[inverter]] publishes on
CAN1, so that one wrong measurement does not close the main contactor
early. The limits and the reaction to a disagreement are not specified yet.

The [[batteries#wiring|battery schematics]] show the contactors, their
coil wiring and the voltage sense leads. The
[[safety#interlock|interlock and safe power schematic]] shows the coil
supply from the VCM.
