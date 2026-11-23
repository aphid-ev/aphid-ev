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

![Schematic of the VCM brake-light output feeding three stop lamps and its reverse-light output feeding the reverse light](/static/generated/wiring/brake_and_reverse_lights.svg#inline "Brake and reverse lights")

The wiring model covers the EV systems and the circuits the conversion
changes, such as the brake lights, which the VCM drives so that
regenerative braking lights them. The VCM also drives the reverse light,
which the regulations require. The car has no gearbox switch for it, so the
VCM lights it while the selected direction is R. The rest of the Beetle's original
electrical system is not modelled.

The harness drawings are on the [[Rear loom]] page.

# Original system {#original-system}

The Beetle's original 12 V system meets the EV systems in three places:

- Its supply is the third MIDI fuse of the
  [[#primary-distribution|primary distribution box]].
- The key keeps switching the original ignition circuits on its original
  feed. The VCM taps the key's run and start terminals as inputs. The car
  has no starter, so the start terminal carries only that signal.
- The stop lamps and the reverse light are fed by the VCM, so their
  original feeds are disconnected. The two brake switches signal the VCM
  and no longer switch the lamps.

# Grounding {#grounding}

The car body is the 12 V return, as in the original Beetle.

- Battery negative is strapped to the body at a ground stud beside the
  battery.
- The VCM, the inverter B-, the OBC/DC-DC signal ground and the rear BMS
  each have their own lead to that stud. The front BMS reaches it through
  the tunnel cable from the rear pack.
- The coolant pump, the radiator fan, the stop lamps and the reverse light
  ground to the body where they sit.
- The DC-DC returns through its case, on a strap straight to battery
  negative; see [[#dcdc|DC/DC]].

The sensors and switches that the VCM reads on a ground of its own keep
that ground: the accelerator sensor and the charge inlet's lock feedback,
LEDs and stop button.

The battery strap is 35 mm², the same as the battery lead.

![Schematic of the ground leads: the battery strap and the ECU grounds on the body's ground stud, and the pump, fan and lamps grounded to the body](/static/generated/wiring/ground_distribution.svg#inline "Ground distribution")

# DC/DC {#dcdc}

The [[obc#dcdc|Mitsubishi DC-DC converter]] delivers up to 1.8 kW, about
125 A at 14.4 V. Its output is a single stud, and the return is the
aluminium case. A 35 mm² cable runs from the stud to the first MIDI fuse of
the primary distribution box, and a 35 mm² ground strap runs from the case
to the battery negative.

# Consumers

The 12 V consumers of the EV systems and their fuses. Lighting and the
other original loads stay on the Beetle's own fuse box.

| Component | Fuse | Box | Supply |
| --- | --- | --- | --- |
| Front BMS | 5 A | Primary | Always on |
| Rear BMS | 5 A | Primary | Always on |
| Gear lever, parking brake switch and brake switch 1 | 5 A | Primary | Always on |
| HV interlock loop | 3 A | Primary | Always on |
| VCM | 15 A | Rear | Always on |
| Inverter B+ | 10 A | Rear | Always on |
| OBC/DC-DC sense | 7.5 A | Rear | Always on |
| Relay coils | 3 A | Rear | Always on |
| Inverter ignition, OBC/DC-DC IGCT and brake switch 2 | 7.5 A | Rear | EV relay |
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

The DC-DC output has a 150 A MIDI fuse and 35 mm² cable, sized for the
converter's full output of about 125 A. The battery lead to the input stud
is 35 mm² as well, because it is not fused.

The rear EV box has a 60 A MIDI fuse and 10 mm² cable. Its fuses add up to
63 A, so check the MIDI rating once the real load currents are measured. The Beetle's original system also has a 60 A MIDI fuse and
10 mm² cable. That rating is a starting value: check it against the
original loads.

The XT also provides four low-current automotive fuse positions through a
four-pin FCI APEX 2.8 connector, housing 54200409. They are always live and
are assigned to:

1. The driver's switches: the gear lever, the parking brake switch and
   brake switch 1. Their outputs connect directly to VCM inputs.
2. Always-on supply for the front BMS.
3. Always-on supply for the rear BMS.
4. The [[safety#interlock|HV interlock loop]], including the emergency
   switch and terminating at the VCM.

The switch fuse is 5 A, the BMS fuses 5 A each and the interlock fuse 3 A.
The connector pin order remains to be decided.

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
sense input of the OBC/DC-DC. The VCM fuse is 15 A, because the one feed
also supplies the VCM's high-side outputs: safe power to the contactor
coils, the stop and reverse lamps, and the inlet lock. It is sized for
incandescent lamps. A fourth fuse feeds the relay stud and so the
relay coils.

The relay-coil fuse is 3 A and the EV-signal fuse 7.5 A. The current the
OBC/DC-DC draws on IGCT is not documented, so check the EV-signal rating on
the bench.

Three relays supply loads that should be off while the car is idle:

- The EV relay feeds the inverter's ignition input, the IGCT input of the
  OBC/DC-DC and brake switch 2. The VCM pulls it in for driving and for
  charging.
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
