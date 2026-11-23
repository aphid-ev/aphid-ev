---
title: Rear loom
category: Wiring
tags: [wiring, electronics]
created: 2026-11-23
---

The rear loom connects the [[vcm|VCM]] to the units in the engine
compartment, to the two battery packs and to the charge inlet.
[[wiring]] explains how to read the drawings.

The fused 12 V feeds, the relay control lines and the interlock leads to the
emergency stop, the inverter and the VCM are not drawn. The fuse boxes and
those interlock ports have no connectors in the model yet. The
[[LV system]] and [[safety]] schematics show them.

# 12 V, cooling and lights

The VCM's cooling and lighting leads. The pump, the fan and the lamps
ground to the body where they sit, on short leads that are not drawn; see
[[LV system#grounding]] and [[waterpump]].

![Harness drawing of the rear LV loom from the VCM to the coolant pump, stop lamps and reverse light](/static/generated/wiring/rear_lv_harness.svg#inline "Rear LV harness")

# VCM, OBC/DC-DC and inverter

CAN1 runs from the VCM through the [[obc|OBC/DC-DC]] to the
[[inverter]]. Each of the three units has its own ground lead to the
ground stud beside the 12 V battery.
The EV-signal lead links the inverter's ignition input and the IGCT input
of the OBC/DC-DC.

![Harness drawing of the CAN1 cables and the EV-signal lead between the VCM, the OBC/DC-DC and the inverter, and their ground leads to the ground stud](/static/generated/wiring/powertrain_signal_harness.svg#inline "Powertrain signal harness")

# CAN

Both buses on one drawing, with every hop as its twisted pair. See
[[EV-CAN#vehicle-buses]].

![Harness drawing of the twisted-pair CAN cables between the BMS controllers, VCM, OBC/DC-DC and inverter](/static/generated/wiring/can_harness.svg#inline "CAN harness")

# Battery signals

The tunnel cable between the two battery packs carries safe power, CAN2,
the interlock and ground. The drawing also shows the interlock lead to the
HV junction box, the rear pack's ground lead to the ground stud and the
CAN2 hop from the rear pack to the VCM. The interlock lead from the emergency stop into the
front pack is not drawn. See [[safety#interlock]].

![Harness drawing of the signal leads at the two battery packs: the tunnel cable, the interlock lead, the ground lead and CAN2 to the VCM](/static/generated/wiring/battery_signal_harness.svg#inline "Battery signal harness")

# Charge inlet

The leads from the VCM's charge connector to the inlet, the lock, the stop
button and the OBC/DC-DC. See [[obc#inlet]].

![Harness drawing of the leads between the VCM charge connector, the inlet, the lock, the stop button and the OBC/DC-DC](/static/generated/wiring/charge_signal_harness.svg#inline "Charge signal harness")
