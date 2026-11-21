---
title: Low voltage electrical system
category: Electronics
tags: [electronics]
created: 2024-08-05
---

Most of the Beetle's original 12 V system remains in place and is
integrated with the new EV systems.

# Auxiliary battery

The car retains a conventional 12 V battery, but it can be substantially
smaller than the Beetle's original battery because it does not crank an
engine. The [[obc#dcdc|Mitsubishi DC-DC converter]] maintains this battery
and supplies the 12 V system whenever the HV system is active. The battery
remains the stable source for always-on loads and for bringing the EV
system online.

# Electrical diagram

![Schematic of the 12 V supply: the DC-DC and the 12 V battery feeding the fuse box](/static/generated/wiring/12v_supply.svg#inline "12 V supply")

![Schematic of the VCM brake-light output feeding three stop lamps](/static/generated/wiring/brake_lights.svg#inline "Brake lights")

## Harness

The rear loom carries the VCM's 12 V, cooling and lighting leads and the
chassis-ground rail.

![Harness drawing of the rear LV loom from the VCM to the coolant pump, radiator fan and stop lamps](/static/generated/wiring/rear_lv_harness.svg#inline "Rear LV harness")

The OBC/DC-DC signal ground and the inverter B- return to the VCM on their
own leads.

![Harness drawing of the ground leads from the OBC/DC-DC and the inverter to the VCM](/static/generated/wiring/signal_ground_leads.svg#inline "Signal ground leads")

# DC/DC

Ground strap to the Mitsubishi OBC/DC-DC.

# Consumers

The 12 V consumers and their fuse ratings. The main power distribution
fuses and relays will be based on this list.

| Component | Fuse | Relay |
| --- | --- | --- |
| Coolant water pump | 10 A | ✓ |
| Cooling fan | 10 A | ✓ |
| EV components | 10 A | ✓ |
| Low beams | 15 A | ✓ |
| High beams | 15 A | ✓ |
| BMS 12 V | 5 A |  |
| VCM 12 V | 5 A |  |

All relays are controlled by an open-drain output of the
[[vcm#io|VCM]].

# Primary distribution {#primary-distribution}

A [Littelfuse XT 880073](https://www.littelfuse.com/products/power-distribution/fuse-only/xt/880073)
is the primary battery-fed distribution box. Its M8 input connects to the
12 V battery. The three MIDI-fused M8 outputs are assigned to:

1. The Mitsubishi DC-DC converter connection.
2. The rear EV fuse and relay box.
3. The Beetle's original 12 V electrical system.

The XT also provides four low-current automotive fuse positions through a
four-pin Aptiv APEX connector. Their provisional assignments are:

1. Ignition key feed; the key outputs connect directly to VCM inputs.
2. Always-on supply for the front BMS.
3. Always-on supply for the rear BMS.
4. The [[safety#interlock|HV interlock loop]], including the emergency
   switch and terminating at the VCM.

These four assignments remain subject to final load, fuse-rating, and
connector-pin decisions.

# Rear EV distribution {#rear-distribution}

A [Littelfuse HWB18 PDM31003ZXM](https://www.littelfuse.com/products/power-distribution/hard-wired-pdms/hwb18/pdm31003zxm)
in the rear engine compartment distributes the MIDI-fused EV feed.
Individually fused always-on outputs supply the VCM and the wake or standby
feeds for the inverter and OBC/DC-DC.

An EV-system relay supplies loads that should be off while the car is idle,
including the inverter and OBC/DC-DC inputs that must remain powered while
their CAN communication is active. A separate relay supplies the electric
cooling fan. The remaining fuse and relay allocation will be defined as the
rear electrical loads are finalized.

# Front power distribution

Fuse/relay box: Littelfuse HWB60-series.
