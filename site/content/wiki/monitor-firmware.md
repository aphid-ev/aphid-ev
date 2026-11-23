---
title: VCM monitor firmware
category: Firmware
tags: [vcm, software, safety]
created: 2024-05-25
---

The firmware of the [[vcm]] monitor microcontroller is
intentionally very simple for two reasons:

- Less code → fewer bugs.
- Less functionality → fewer updates needed.

The application monitors a few different things:

- State of the application in the main microcontroller, through a serial
  bus.
- Accelerator pedal inputs.
- Brake pedal input.
- Torque request sent from the application in the main microcontroller to
  the inverter over [[EV-CAN]].
- Motor rotational speed reported by the inverter.

Its only task is to control the high side output of the VCM and disable
it in case of any faults. This output supplies the coils of the high
voltage contactors. The monitor enables it only while the
[[safety#interlock|HV interlock loop]], which includes the emergency
switch, is valid. The [[bms|BMS]] controllers switch the low side of the
same coils. A contactor closes only when both the monitor and the BMS
allow it, as described under [[contactors#roles]].

The firmware is written in Rust. The binary is in
[`ecus/vcm/firmware/monitor`](https://github.com/aphid-ev/aphid-ev/tree/main/ecus/vcm/firmware/monitor), and the
checks it runs are in [`crates/vcm-core`](https://github.com/aphid-ev/aphid-ev/tree/main/crates/vcm-core). So far
the binary only starts up; the throttle and torque request checks exist
in the crate but are not wired to the hardware.

# States

The firmware has only three states:

```mermaid
stateDiagram-v2
    [*] --> Init: Reset
    Init --> Active: Main application requests activation, interlock closed
    Active --> Init: Main application releases safe power
    Active --> Fault: A fault occurs
    Fault --> [*]: Next key cycle
```

## Init

`High side output: off`

After reset, the firmware starts in the `Init` state. All peripherals and
the application are initialized, and then it awaits an activation request
from the main microcontroller.

## Active

`High side output: on`

After the activation request has been received, the high side driver is
activated and all safety-related signals are monitored. The firmware
refuses the request while the [[safety#interlock|HV interlock loop]] is
open.

When the main application releases safe power, at key off or at the end of
a charge, the firmware switches the high side output off and returns to
`Init`. That is a normal shutdown, not a fault. The
[[contactors#opening|opening sequence]] describes when it happens.

## Fault

`High side output: off`

If a fault occurs, the application enters the fault state and the high
side driver is deactivated. The monitor application remains in this state
until the next key cycle, when the VCM goes to sleep and wakes again.

# Faults

The following conditions take the application from the active state to
the fault state.

## HV interlock loop open

If the [[safety#interlock|HV interlock loop]] opens, this fault is
triggered at once.

## Main application unresponsive

If the main application fails to respond within **100 ms** to a status
request on the serial communication, this fault is triggered.

## Faulty accelerator signal

If the dual-channel accelerator pedal inputs are not within **10 %** of
each other, this fault is triggered.

## Faulty accelerator supply

The VCM supplies 5 V to the accelerator hall-effect sensors. If this is
outside of **±10 %**, a fault is raised.

## Faulty torque request

If torque is outside of **10 %** of the accelerator pedal input for more
than **200 ms**, or if accelerating torque is requested while **braking**.

## Faulty direction change

If the direction is changed from forward to reverse or vice versa while
motor speed is above *TBD* rpm.
