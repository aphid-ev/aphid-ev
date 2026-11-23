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

The hardware and firmware are in the
[aphid-ev](https://github.com/aphid-ev/aphid-ev) repository. The board is
in [`ecus/vcm/hardware`](https://github.com/aphid-ev/aphid-ev/tree/main/ecus/vcm/hardware), the firmware for both
microcontrollers in [`ecus/vcm/firmware`](https://github.com/aphid-ev/aphid-ev/tree/main/ecus/vcm/firmware), and the
logic they share in [`crates/vcm-core`](https://github.com/aphid-ev/aphid-ev/tree/main/crates/vcm-core) and
[`crates/ev-can`](https://github.com/aphid-ev/aphid-ev/tree/main/crates/ev-can).

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

- **Off:** the main contactors and EV-system relay are open. The VCM
  sleeps on its always-on supply and the BMS controllers stay powered. The
  key or a charge cable on the proximity pilot wakes the VCM.
- **Ready:** the control networks and switched EV loads are powered. After
  safety checks, the system performs precharge and closes the main
  contactors; propulsion remains at zero torque until a direction is
  accepted.
- **Drive:** a direction has been accepted with the brake applied, the
  charge connector is absent, and the VCM may command traction or
  regenerative torque.
- **Charge:** PP detection initiates the sequence, also with the key off.
  The VCM inhibits drive, locks the inlet and pulls in the EV relay, which
  powers the OBC and the coolant pump. The contactors close through the
  normal precharge sequence, and the OBC derives the permitted current from
  CP. When the charge is finished the VCM unlocks the inlet, opens the
  contactors and goes back to sleep.
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

The main firmware runs on an STM32G491 with 512 KB of flash, with a
separate STM32G431 monitoring it. Every DIY ECU uses the STM32G4 family,
so one bootloader and one HAL cover the whole car. The main MCU is the same
512 KB part as the [[bms]] controllers, so they share one flash layout.

The STM32G474 was considered for its dual-bank flash, which would let an
update be written to the idle bank and swapped in once checked. Firmware is
flashed over CAN in the garage, and an ECU whose update fails stays in the
bootloader until it is flashed again, so bank swap is not worth a second
part. The G474's other extras, such as the high-resolution timer and the
extra ADCs, are for power conversion, which the VCM does not do.

The monitor is a separate chip, with its own supply and clock, rather than
a second core on the main MCU's die. Two cores on one die share the supply,
the clock, the reset and the CAN peripheral, so a single hardware fault
could stop both the controller and its supervisor.

The current schematic still has the earlier choice, an STM32F405RG with an
STM32F091CC monitor.

## I/O {#io}

> [!WARNING]
> This pinout predates the current electrical architecture. It does not
> yet have the charge inlet proximity pilot, lock, LED and stop button
> signals, the coolant pump PWM and tacho, or the radiator fan relay
> output.

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

The wiring model is the current list of signals. The drawing shows every
VCM port and what it connects to, and is the starting point for the I/O of
the next board revision. A name that starts with `ic_` is a branch
connector on the way to the device.

![Schematic of the VCM with every port and a label naming what it connects to](/static/generated/wiring/vcm_i_o.svg#inline "VCM I/O")

The model has 35 VCM signals. It does not yet have the reverse lights. Its
connectors are placeholders,
one per function, and the interlock input, the safe power output and the EV
relay output have no connector pin yet.

The ModICE enclosure size is not chosen. It comes with 32 or 40 connector
positions, and the choice waits for the final signal count.

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
switch fuse in the
[[LV system#primary-distribution|primary distribution box]], which also
feeds the gear lever, the parking brake switch and brake switch 1.

+12 V when the ignition switch is in the ignition (RUN) position.

### Crank

+12 V when the ignition switch is in the crank (START) position.

The key hangs off the cab loom on its own branch, so the key loom unplugs as one.

![Harness drawing of the cab-loom side of the key branch: the VCM key connector to the inline connector](/static/generated/wiring/cab_loom_key_branch.svg#inline "Cab loom, key branch")

![Harness drawing of the key loom from the inline connector to the key switch](/static/generated/wiring/key_loom.svg#inline "Key loom")

### Accelerator input (ACC1/ACC2)

The accelerator pedal uses a dual-channel analog position sensor so the
VCM can cross-check the two signals. See [[accelerator]] for the signal
levels and the sensor.

![Harness drawing of the cab-loom side of the pedal branch: the VCM accelerator connector to the inline connector](/static/generated/wiring/cab_loom_pedal_branch.svg#inline "Cab loom, pedal branch")

![Harness drawing of the pedal stub from the inline connector to the accelerator pedal](/static/generated/wiring/pedal_stub.svg#inline "Pedal stub")

### Brake input (BRAKE1/BRAKE2)

The original brake switches connect independently to the VCM. Brake input
requests regenerative braking, activates the brake lights, and prevents
positive accelerator torque while the brakes are applied. Each switch has
its own 12 V feed; see [[LV system#brake-switch-feeds]].

![Harness drawing of the cab-loom side of the brake branch: the VCM brake connector to the inline connector](/static/generated/wiring/cab_loom_brake_branch.svg#inline "Cab loom, brake branch")

![Harness drawing of the brake loom from the inline connector to the two brake sensors](/static/generated/wiring/brake_loom.svg#inline "Brake loom")

### Direction input (DIR_F1/DIR_F2 and DIR_R1/DIR_R2)

Input from the [[direction]]: two complementary switch lines per direction
from the sprung gear lever. Each pull steps the state along R, N, D. The
VCM accepts a step into D or R only while the car is stopped with the brake
applied. The parking brake switch uses a fifth input on the same connector.

![Harness drawing of the cab-loom side of the gear branch: the VCM gear connector to the inline connector](/static/generated/wiring/cab_loom_gear_branch.svg#inline "Cab loom, gear branch")

![Harness drawing of the gear loom from the inline connector to the gear lever](/static/generated/wiring/gear_loom.svg#inline "Gear loom")

### Charge inlet

Seven signals serve the [[obc#inlet|charge inlet]], besides the DC-DC
enable to the OBC:

- The proximity pilot input. It is also a wake source, and reading it as an
  analogue value gives the cable's current rating.
- Two H-bridge outputs for the [[obc#lock|inlet lock]] motor, which draws
  at most 650 mA for up to one second.
- The lock feedback input, a switch closed to ground while locked.
- Two outputs for the red and green status LEDs. The green LED pulses
  while charging, so its output needs PWM or a software blink.
- The stop button input, a switch closed to ground.

The feedback switch, the LEDs and the button share one ground return pin.
The lock is optional, so the car has to charge with the lock pins left
open.

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

The [[bootloader|CAN bootloader]] the other DIY ECUs use does not support
the STM32F405 or the STM32F091 yet.

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
