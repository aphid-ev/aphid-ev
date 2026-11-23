---
title: High voltage electrical system
category: Electronics
tags: [battery, electronics]
created: 2024-08-05
---

The conversion is based on a Nissan Leaf drivetrain, with selected
subsystems replaced by custom or Mitsubishi components.

In a more modern EV there are usually several loads driven by the high
voltage system — AC compressor, electrical heater and so on. In the
Aphid there are two consumers: the
[[obc|Mitsubishi Outlander OBC/DC-DC]] and the
[[inverter|traction inverter]].

The pack itself is described under [[batteries]] and is gated by the
[[contactors]].

# Battery string and fusing

The front and rear [[batteries|battery sections]] are connected in series
to form a 96s2p traction pack. The MSD in the front battery contains the
traction pack's 250 A main fuse.

The HV junction box distributes the completed battery string to the
inverter and OBC/DC-DC. The OBC/DC-DC branch has its own 30 A fuse on the
positive leg in the junction box, the rating Mitsubishi uses for the unit,
and runs in 6 mm² cable. The fuse type remains to be selected. The branch
has no contactor of its own, so the whole HV bus,
inverter included, is live while the car charges.

# Isolation

The traction HV system is floating, with no on-board isolation monitor.
[[safety#electrical-safety]] covers isolation testing and the HV interlock
loop.

# Electrical diagram

The schematic follows the HV bus from the battery string through the MSD and
the junction box to the inverter, motor and OBC/DC-DC. The interlock loop runs
along the bottom.

![Schematic of the HV system: battery string, MSD, HV junction box, inverter, motor and OBC/DC-DC](/static/generated/wiring/hv_system_overview.svg#inline "HV system overview")

## Harness

![Harness drawing of the HV cables between the batteries, MSD, junction box, inverter and OBC/DC-DC](/static/generated/wiring/main_hv_harness.svg#inline "Main HV harness")
