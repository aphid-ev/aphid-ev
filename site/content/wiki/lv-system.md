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

![Schematic of the 12 V supply: the battery and the DC-DC on the primary fuse box, which feeds the rear EV fuse box](/static/generated/wiring/12v_supply.svg#inline "12 V supply")

![Schematic of the VCM brake-light output feeding three stop lamps](/static/generated/wiring/brake_lights.svg#inline "Brake lights")

The wiring model covers the EV systems and the circuits the conversion
changes, such as the brake lights, which the VCM drives so that
regenerative braking lights them. The rest of the Beetle's original
electrical system is not modelled.

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

The 12 V consumers of the EV systems and their fuses. Lighting and the
other original loads stay on the Beetle's own fuse box.

| Component | Fuse | Box | Supply |
| --- | --- | --- | --- |
| Front BMS | 5 A | Primary | Always on |
| Rear BMS | 5 A | Primary | Always on |
| Key, gear lever, parking brake switch and brake switch 1 | *TBD* | Primary | Always on |
| HV interlock loop | *TBD* | Primary | Always on |
| VCM | 5 A | Rear | Always on |
| Inverter B+ | 10 A | Rear | Always on |
| OBC/DC-DC sense | *TBD* | Rear | Always on |
| Relay coils | *TBD* | Rear | Always on |
| Inverter ignition, OBC/DC-DC IGCT and brake switch 2 | *TBD* | Rear | EV relay |
| Coolant water pump | 10 A | Rear | Pump relay |
| Cooling fan | 10 A | Rear | Fan relay |

The VCM switches the relay coils on the low side. One output pulls in the
EV relay and the pump relay together, and a second output pulls in the fan
relay.

# Primary distribution {#primary-distribution}

A [Littelfuse XT 880073](https://www.littelfuse.com/products/power-distribution/fuse-only/xt/880073)
is the primary battery-fed distribution box. It sits under the rear seat
beside the 12 V battery, the Beetle's original battery position. The battery
connects to the M8 input stud, which is rated for 300 A and is not fused, so
the lead between them is kept short. The battery terminals and the studs
need covers so the seat frame cannot short them.

![Schematic of the primary fuse box: one bus bar feeding three MIDI fuses and four blade fuses](/static/generated/wiring/primary_fuse_box.svg#inline "Primary fuse box")

The three MIDI-fused M8 outputs are assigned to:

1. The Mitsubishi DC-DC converter connection.
2. The rear EV fuse and relay box.
3. The Beetle's original 12 V electrical system.

The MIDI fuse ratings and the cable sizes are not chosen yet. The DC-DC
cable has to carry the converter's full output current.

The XT also provides four low-current automotive fuse positions through a
four-pin FCI APEX 2.8 connector, housing 54200409. They are always live and
are assigned to:

1. The driver's switches: the ignition key, the gear lever, the parking
   brake switch and brake switch 1. Their outputs connect directly to VCM inputs.
2. Always-on supply for the front BMS.
3. Always-on supply for the rear BMS.
4. The [[safety#interlock|HV interlock loop]], including the emergency
   switch and terminating at the VCM.

The fuse ratings and the connector pin order remain to be decided.

# Rear EV distribution {#rear-distribution}

An [Eaton Bussmann 15300 series RTMR](https://static.datasheets.com/doc/12245678-eaton-15303-4-0-4-ds.pdf)
with the micro relay base, part 15304-2-2-4, sits in the rear engine
compartment and distributes the MIDI-fused EV feed. It holds ten MINI fuses
and five micro relays on Metri-Pack 280 terminals. Facts from the Eaton
datasheet:

- The feed connects to an M6 stud that is bussed to one side of every fuse.
  The stud is rated for 80 A.
- A second M6 stud is bussed to coil pin 86 of every relay.
- Each output terminal is rated for 30 A and takes 12 to 22 AWG wire.
- The box is sealed to IP66 with its cover, cable seals and cavity plugs
  fitted, and is rated from -40 °C to 125 °C.

A hard-wired box without a bus bar, such as the Littelfuse HWB18 or HWB60,
was rejected. Its terminals take at most 5 mm² wire, so the feed would have
to fan out through thinner leads that only the MIDI fuse protects.

![Schematic of the rear EV fuse box: seven fuses on one bus, three of them feeding a relay each](/static/generated/wiring/rear_ev_fuse_box.svg#inline "Rear EV fuse box")

Every fuse is always live, so a switched load goes through its fuse, then
its relay, then to the load. A short jumper at the back of the box links
each of those fuses to pin 30 of its relay.

Three fuses supply always-on loads: the VCM, the inverter's B+ and the
sense input of the OBC/DC-DC. A fourth fuse feeds the relay stud and so the
relay coils.

Three relays supply loads that should be off while the car is idle:

- The EV relay feeds the inverter's ignition input, the IGCT input of the
  OBC/DC-DC and brake switch 2.
- The pump relay feeds the coolant pump. It shares the EV relay's control
  line.
- The fan relay feeds the electric cooling fan.

Three fuse positions and two relay positions are spare.

> [!NOTE]
> The datasheet says the relay stud is bussed to pin 86. Check on the 2D
> print that it does not also feed pin 30 before ordering.

# Brake switch feeds {#brake-switch-feeds}

The two brake switches take their 12 V from different fuses in different
boxes. Brake switch 1 is on the always-on switch fuse in the primary box.
Brake switch 2 is behind the EV relay in the rear box. A blown fuse or a
broken feed then silences one channel only, and the VCM sees the two
channels disagree.
