---
title: Safety architecture
category: Architecture
tags: [safety, architecture]
created: 2024-05-30
---

The safety systems are designed to protect against two main cases:

- Unexpected torque from the motor leading to unwanted acceleration or
  braking.
- Access to the high voltage rails.

The same method is used to reach a safe state for both:

> [!IMPORTANT]
> Opening the battery contactors will both prevent the inverter/motor
> producing torque **and** cut access to high voltage components.

The general idea is to have a dual-channel system for all safety functions
such that **a single error will never cause a dangerous situation** and
**a single error is always detected**.

*TBD: Safety architecture drawing.*

# Electrical safety

The traction HV system is isolated from the vehicle chassis and operates as
a floating system. No active on-board isolation monitor is currently
planned. Insulation resistance will be checked with a suitable insulation
tester during construction, commissioning, and service.

## HV interlock loop {#interlock}

A high-voltage interlock loop passes through the HV components and
enclosures, except the [[obc|Mitsubishi OBC/DC-DC]], and includes a
driver-accessible emergency switch. A valid loop permits the secondary
processor in the VCM to enable a high-side driver that supplies the
battery contactor coils. The BMS controllers independently complete the
coils' low-side paths, so contactors require authorization from both the
VCM safety path and the relevant BMS.

The loop is fed from a low-current fuse in the primary distribution box
and terminates at the VCM; see [[LV system#primary-distribution]].

Inside the front pack the loop passes through the MSD, so pulling the MSD
opens it. The schematic shows the loop from its fuse to the VCM, and the safe power
the VCM feeds to the contactor coils in both packs while the loop is closed.

![Schematic of the interlock loop from the primary fuse box through the emergency stop, the battery packs, HV junction box and inverter to the VCM, and the safe power from the VCM to both packs](/static/generated/wiring/interlock_and_safe_power.svg#inline "Interlock and safe power")

The harness drawing of the interlock leads at the battery packs is on the
[[Rear loom#battery-signals|Rear loom]] page.

# Functional safety

The VCM's secondary processor independently monitors the accelerator
channels, brake inputs, and torque requests sent by the primary processor.
Those checks and the internal processor architecture belong to the VCM
design rather than the vehicle harness. At the external electrical
interface, the secondary processor controls the high-side driver supplying
the battery contactors and removes contactor power as a last resort if the
main processor stops responding on their internal communication bus or
another shutdown condition is detected.

The [[vcm]] and [[Monitor firmware]] pages describe the VCM side. The
vehicle-level fault behaviour is described under
[[vcm#operating-states|operating states]].

# Unintended acceleration or braking

There is a lot of monitoring around making sure the car does not get any
unintended acceleration or braking. As a last resort, the driver-accessible
emergency switch is part of the [[#interlock|HV interlock loop]]. Pushing
it breaks the loop, and the VCM secondary processor removes the high-side
supply to the contactor coils.

## Definitions

### Torque

Motor torque is defined as positive in the direction of forward
acceleration and negative in the direction of reverse acceleration. Here
we only consider motor torque and assume the transmission is in a forward
gear, meaning wheel torque has the same sign as motor torque.

### Speed

Speed is defined in the same direction as torque: a positive speed means
the car is moving forward, negative means it is moving in reverse.

### Accelerating torque

Acceleration is torque in the direction the car is currently moving — in
other words, torque that works on increasing `abs(speed)`.

### Braking torque

Braking is torque in the direction opposite to which the car is currently
moving — torque that works on decreasing `abs(speed)`.

## Error cases

The table below defines identified safety issues that could lead to
unwanted acceleration or braking and the mitigations in place to manage
each.

### Accelerator pedal

Faults in the accelerator pedal sensors or in the interpretation of them
could cause unintended torque requests to be sent from the VCM to the
inverter.

#### Too low/high supply voltage to accelerator sensor

**Monitored:** VCM monitor and main application.

**Action:** Set fault condition in VCM main app, disable main contactor
high-side supply if accelerating torque is requested.

#### Bad accelerator pedal sensor

**Monitored:** Dual independent sensor channels monitored by VCM monitor
and main application.

**Action:** Set fault condition in VCM main app, disable main contactor
high-side supply if accelerating torque is requested.

#### VCM main application reads wrong accelerator position

**Monitored:** VCM monitor application compares accelerator sensor input
with torque request.

**Action:** Set fault condition in VCM main app, disable main contactor
high-side supply if accelerating torque is requested.

### VCM main application faults

There can be many faults in the main application; most are caught by the
torque request supervision.

#### Torque request while braking

**Monitored:** VCM monitor application.

**Action:** Set fault condition in VCM main app, disable main contactor
high-side supply if accelerating torque is requested.

### Inverter faults

Since the inverter is an unmodified commercial part, that has been shown
to be very reliable, it is generally considered to be safe. There is some
supervision regardless.

# Access to high voltage

This error case is mainly relevant when servicing the drivetrain
components. To prevent access to high voltage busbars and similar while
they are powered, the built-in interlock circuit of the
[[inverter#interlock|inverter]] is connected in series with the
[[#interlock|HV interlock loop]]. The Mitsubishi OBC/DC-DC has no
interlock functionality and is not part of the loop.
