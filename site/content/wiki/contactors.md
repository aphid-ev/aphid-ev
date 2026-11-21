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

## Auxiliary contacts

Their mechanically linked normally-open auxiliary contacts provide actual
contact state for welded-contactor detection. The receiving ECU and
connector-pin allocation remain to be defined.

# Coil control

Contactors require authorization from both the VCM safety path and the
relevant BMS:

- The secondary safety processor in the [[vcm|VCM]] enables a high-side
  driver that supplies the battery contactor coils, as long as the
  [[safety#interlock|HV interlock loop]] is valid.
- Both [[bms|BMS]] controllers switch the low side of their main
  contactor coils. The front BMS also controls the precharge contactor.

# Precharge

Precharge is managed by a Littelfuse `DCNSEV30-B` in series with a
`10 Ω 100 W` power resistor. These are connected in parallel to the
positive main contactor in the [[batteries#front-battery|front battery]].

The [[inverter]] measures the HV bus voltage and publishes it over CAN1.
Its DC-link voltage measurement is used to determine when precharge is
complete.

*TBD: schematic.*
