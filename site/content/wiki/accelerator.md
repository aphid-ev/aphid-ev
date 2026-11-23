---
title: Accelerator pedal sensor
category: Components
tags: [hardware, electronics]
created: 2026-11-23
---

The accelerator position reaches the [[vcm#io|VCM]] as two analog signals
from one sensor. The electrical interface is fixed. The sensor behind it is
chosen but not yet built.

# Electrical interface

| Signal | Level |
| --- | --- |
| 5 V | Sensor supply from the VCM |
| GND | Sensor ground at the VCM |
| APPS1 | 0.5 V at rest, rising to 4.5 V at full pedal |
| APPS2 | 4.5 V at rest, falling to 0.5 V at full pedal |

Both outputs are ratiometric to the 5 V supply. Because one rises while the
other falls, the two always add up to the supply voltage. A drifting supply,
a lost ground or a short between the two signals breaks that sum. A signal
below 0.5 V or above 4.5 V means an open or shorted wire.

The [[Monitor firmware]] compares the two channels after inverting the
second.

# Sensor options

**Original pedal with a cable-driven sensor.** This is the preferred
option. The Beetle's original pedal and throttle cable stay. In the engine
compartment the cable turns a Piher PSC-360 rotary sensor instead of a
carburettor lever. The pedal looks original and the sensor leads to the VCM
are short. The cable, a lever and the return springs have to be built.

**Off-the-shelf pedal.** A complete pedal assembly from a production EV or
hybrid, such as a Tesla or a Prius, replaces the original pedal. It has two
sensor channels and its return springs built in, and needs no cable. It
does not look original, and its leads run the length of the car.

Either option uses the interface above, so the wiring model and the VCM do
not depend on the choice.

# Piher PSC-360

Facts from the Piher PSC-360 datasheet, revision 260225:

- Hall-effect rotary sensor with a 6 mm shaft and endless rotation. The
  magnet on the shaft is the only moving part.
- The redundant version has two independent analog outputs on four leads:
  supply, ground, signal 1 and signal 2. It draws 17 mA from 5 V ±10 %.
- The outputs are ratiometric and swing between 10 % and 90 % of the
  supply. The output function for redundant versions has one rising and
  one falling output.
- The electrical angle is ordered between 40° and 360°.
- Linearity is ±1 %, resolution up to 12 bit.
- Operating temperature is -40 °C to 125 °C.
- The sensor has 245 mm fly leads of 0.35 mm² and no connector. Brown is
  the supply, blue the ground, black signal 1 and white signal 2.
- A version with a lever on the shaft exists, and a mounting plate can be
  ordered with it.
- Ferromagnetic material close to the sensor can affect its linearity.

The full-redundant version has separate supply and ground leads for each
output, six leads in total. The VCM has one accelerator supply, so the
four-lead redundant version fits.

# To decide and build

- The electrical angle, chosen from the lever length and the cable travel.
- Two return springs at the sensor lever, so the sensor goes to rest if the
  cable breaks or one spring fails.
- A pedal stop that keeps the lever inside the ordered angle.
- A mount that keeps steel parts away from the sensor body.
- The connector on the fly leads.

A cable that sticks holds the pedal position. The VCM blocks positive
torque while the brakes are applied, which covers that case.
