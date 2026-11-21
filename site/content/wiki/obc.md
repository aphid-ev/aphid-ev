---
title: Mitsubishi Outlander OBC/DC-DC
category: Components
tags: [charging, dcdc, electrical]
created: 2026-11-21
---

The original plan used a [[pdm|Nissan Leaf power delivery module]], but it
does not fit the available space. Charging and low-voltage conversion
therefore use a more easily packaged combined on-board charger (OBC) and
DC-DC converter from a Mitsubishi Outlander. The Outlander unit has no
interlock functionality and is excluded from the
[[safety#interlock|HV interlock loop]]. The OBC charges the traction
battery from single-phase AC, while the DC-DC converter supplies the 12 V
system and maintains the auxiliary battery. The VCM communicates with and
enables the unit through CAN1 and discrete control signals.

# HV connection

The unit has its own fused branch from the HV junction box. The
[[HV system#battery-string-and-fusing|HV system]] page describes the
fusing.

# 12 V connections

The DC-DC output connects to the first MIDI-fused output of the primary
distribution box. The wake or standby feed and the switched input of the
unit come from the rear EV distribution box. Both boxes are described in
the [[LV system]].

# CAN interface

The unit is an unterminated intermediate node on CAN1, between the VCM
and the inverter. See [[EV-CAN#vehicle-buses]].

# DC-DC {#dcdc}

The Mitsubishi DC-DC converter maintains the auxiliary 12 V battery and
supplies the 12 V system whenever the HV system is active, in a similar
fashion to the alternator of a car with a combustion engine.

# Charge inlet {#inlet}

The current assumption is a Type 2 inlet supporting single-phase AC
charging at a maximum of 16 A. L1, neutral, and protective earth connect to
the Mitsubishi OBC; L2 and L3 are unused. The proximity pilot (PP) connects
to the VCM and initiates the charging sequence. The control pilot (CP)
connects directly to the OBC, which uses it to regulate charging to the
current advertised by the charging point.

The inlet has red and green status LEDs and a locking bolt controlled by
the VCM. The exact inlet part, LED meanings, actuator wiring, and any
lock-position feedback remain to be selected.

The charge sequence is described under
[[vcm#operating-states|operating states]].
