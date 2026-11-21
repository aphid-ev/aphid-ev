---
title: DIY VCM
category: Components
tags: [canbus, vcm, electrical, software]
created: 2024-05-20
---

While re-using all of the power electronics from the donor cars makes
sense, the plan is to design a custom version of what Nissan call a VCM
(Vehicle Control Module) — and the same nomenclature is used here. The
VCM is the control unit that manages all the main functions of the
vehicle and coordinates the other components.

The DIY VCM is the main vehicle ECU. It uses two processors: a primary
processor for vehicle control and a secondary safety processor for
independent supervision. The VCM handles driver inputs, torque requests,
inverter and charger communication, contactor power authorization, cooling,
and brake-light control.

The easiest way to follow development is through the
[vehicle-controller](https://github.com/aphid-ev/vehicle-controller)
GitHub repository, which contains both the hardware and the firmware.

See also: [[VCM requirements]] for the design targets.

# Safety

Since this VCM controls a car moving at high speeds and interacts with an
electrical system containing dangerous voltages, safety is very important.
There are two major things that need to be managed by the safety system:

- Traction torque
- High voltages

See [[safety]] for the full system view.

## Safe state

In case of a critical failure, the car is taken to a safe state by
commanding zero torque and, for faults requiring HV shutdown, opening the
battery contactors. This prevents the motor/inverter from producing torque
and removes the risk of electrical injury. See the fault state under
[[#operating-states]].

## VCM safety design

The VCM has dual microcontrollers: one runs the main application, the
other monitors critical signals and verifies that the main application is
executing as expected. The two communicate over a serial link.

- Both MCUs read and cross-check the dual-channel input from the
  accelerator pedal.
- The monitor MCU has read access to the EV-CAN bus and verifies the
  torque request from the main application is reasonable.
- The monitor MCU controls the high-side switch for the precharge and
  main contactors, enabled only while the
  [[safety#interlock|HV interlock loop]] is valid.
- The [[bms|BMS]] controllers, not the VCM, switch the low side of the
  contactor coils. See [[contactors#coil-control]].
- The main MCU sends watchdog counters over the serial port.
- The monitor MCU can disable the EV-CAN transceiver and, as a final
  option, reset the main MCU.

# Operating states {#operating-states}

- **Off:** the main contactors and EV-system relay are open. The BMS
  controllers and the VCM's low-power supply remain on for monitoring and
  wake-up.
- **Ready:** the control networks and switched EV loads are powered. After
  safety checks, the system performs precharge and closes the main
  contactors; propulsion remains at zero torque until a direction is
  accepted.
- **Drive:** a direction has been accepted with the brake applied, the
  charge connector is absent, and the VCM may command traction or
  regenerative torque.
- **Charge:** PP detection initiates the sequence, the VCM inhibits drive
  and locks the inlet, and the contactors close through the normal
  precharge sequence. The OBC derives the permitted current from CP.
- **Fault:** torque is commanded to zero. For faults requiring HV shutdown,
  or if the main VCM processor stops responding on the internal
  communication bus, the safety processor removes the high-side
  contactor-coil supply so the contactors open.

The [[contactors]] page describes precharge, and the
[[obc#inlet|charge inlet]] section describes the PP and CP signals.

# Hardware

The PCB is created in the open source [KiCad](https://www.kicad.org/)
software.

## Enclosure

The enclosure is the pretty robust Cinch ModICE ME-MX 24. See
[[VCM requirements#enclosure]] for part numbers. The front and rear
[[bms#packaging|BMS controllers]] use the same ModICE enclosures with
Molex MX-150 connectors.

## Microcontrollers

The main firmware runs on an STM32F405RB microcontroller, with a smaller
STM32F091CC monitoring it.

## I/O {#io}

> [!WARNING]
> This pinout predates the current electrical architecture. It does not
> yet have the charge inlet proximity pilot, lock and LED signals, the
> coolant pump PWM and tacho, or the radiator fan relay output.

The VCM has two 12-pin Molex MX-150 connectors:

| Pin | Left | Right |
| --- | --- | --- |
| 1 | GND | GND |
| 2 | CAN1 Low | CAN2 Low |
| 3 | CAN1 High | CAN2 High |
| 4 | BRAKE1 | *TBD* |
| 5 | BRAKE2 | *TBD* |
| 6 | +12 V IN | CRANK |
| 7 | GND | GND |
| 8 | *TBD* | High side out |
| 9 | EXT 5V | EXT 5V |
| 10 | ACC1 | DIR_R2 |
| 11 | ACC2 | DIR_R1 |
| 12 | +12 V IN | IGNITION |

See [[VCM requirements#i-o]] for the canonical connector pinout.

### CAN1 (high/low)

Used to communicate with [[EV-CAN]] nodes: the Nissan Leaf inverter and
the [[obc|Mitsubishi OBC/DC-DC]]. Available to both the main and the
monitor microcontroller.

### CAN2 (high/low)

Used to communicate with the DIY vehicle systems: the front and rear
[[bms|BMS]] controllers, and future DIY nodes such as a driver display.
Only available to the main controller. See [[EV-CAN#vehicle-buses]].

### High side out

High side driver providing +12 V to the high side of the main and
precharge high voltage contactors.

### +12 V IN

Main feed from the 12 V battery.

### Ignition

The original-style key switch receives a fused 12 V feed and sends separate
RUN and START signals directly to VCM inputs. The key feed comes from the
[[LV system#primary-distribution|primary distribution box]].

+12 V when the ignition switch is in the ignition (RUN) position.

### Crank

+12 V when the ignition switch is in the crank (START) position.

### Accelerator input (ACC1/ACC2)

The accelerator pedal uses a dual-channel analog position sensor so the
VCM can cross-check the two signals.

### Brake input (BRAKE1/BRAKE2)

The original brake switches connect independently to the VCM. Brake input
requests regenerative braking, activates the brake lights, and prevents
positive accelerator torque while the brakes are applied.

### Direction input (DIR_F1/DIR_F2 and DIR_R1/DIR_R2)

Input from the [[direction]]: momentary reverse and drive switches mounted
on the gear lever. The VCM accepts a change only while the car is stopped
with the brake applied, and no active request represents neutral.

### External 5 V feed

5 V output to accelerator pedal hall effect sensors and the momentary
switches on the brake pedal and direction selector.

# Firmware

The firmware is developed in [Rust](https://www.rust-lang.org/), both
because it provides a lot of nice safety features and because it is
pleasant to work with.

## Bootloader

At some point a CAN-based bootloader will be created for both
microcontrollers so they can be updated without opening the enclosure.
One option is to use the built-in ROM bootloader in the STM32 parts —
they are wired so that the ROM bootloader CAN bus is on CAN1 and CAN2
respectively.

## Monitor application

The monitoring microcontroller runs a fairly simple application that
monitors the main application and can disable the high voltage contactors
if something goes wrong. See [[Monitor firmware]].

## Main application

See [[Main firmware]].

# Notes

High side drivers: ST VNQ9025AT

- Waterpump relay
- Cooling fan relay
- Brake lights
- Reverse lights
