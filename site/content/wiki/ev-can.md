---
title: Nissan Leaf EV-CAN
category: Software
tags: [canbus, leaf, pdm, electrical, software]
created: 2024-05-19
---

The EV-CAN network is the main CAN bus used by the donor Nissan Leaf
components. In the Aphid it is CAN1, the OEM powertrain bus that the
[[vcm|DIY VCM]] uses to talk to the [[inverter|traction inverter]] and the
[[obc|Mitsubishi OBC/DC-DC]]. The DIY ECUs use a second bus, CAN2.

# Vehicle buses {#vehicle-buses}

The VCM provides two independent CAN interfaces:

- **CAN1 - OEM powertrain:** connects the VCM, Nissan Leaf inverter, and
  Mitsubishi OBC/DC-DC. The secondary safety processor monitors this bus,
  including outgoing torque requests.
- **CAN2 - DIY vehicle systems:** connects the VCM to the front and rear
  [[bms|BMS]] controllers. Future DIY nodes, such as a driver display, may
  also use this bus.

Both buses use twisted pairs. CAN1 runs VCM to OBC/DC-DC to inverter and is
terminated with 120 ohm resistors at the VCM and inverter; the OBC/DC-DC is
an unterminated intermediate node. CAN2 runs front BMS to rear BMS to VCM
and is terminated at the front BMS and VCM. Bus bit rates remain to be
documented.

## Wiring

![Schematic of CAN1 and CAN2 with the VCM between them and the terminated nodes marked](/static/generated/wiring/can_bus.svg#inline "CAN buses")

The harness drawing of the CAN cables is on the [[Rear loom#can|Rear loom]]
page.

# Electrical

As any CAN bus, the EV-CAN network uses a twisted pair for differential
signalling to improve noise immunity.

## Signalling

In the Leaf, EV-CAN uses ISO 11898-1 with 11-bit identifiers and a 500 kbit/s baud rate.

## Topology

A well-functioning CAN bus should have a linear topology with short stubs
sticking out and a 120 Ω termination at each end. The order and
terminations of CAN1 and CAN2 in the Aphid are listed under
[[#vehicle-buses]].

# Data integrity

Some critical frames are protected with both sequence counters and a CRC
checksum (in addition to the CRC built into the CAN frames themselves).

## Sequence counter

The sequence counter just counts up from `0 → 3` continuously.

## CRC checksum

There is a specific Nissan CRC calculation of the first 7 bytes of the
CAN frame stored in the 8th byte. It is a standard CRC8 calculation with
the initial value `0x00` and the polynomial `0x85`.

```rust
fn nissan_crc(data: &[u8]) -> u8 {
    const POLYNOMIAL: u8 = 0x85;
    let mut crc = 0u8;

    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ POLYNOMIAL
            } else {
                crc << 1
            };
        }
    }

    crc
}
```

Over `6E 6E 00 04 03 06 E0` it gives `0x69`. That frame and its CRC come
from dalathegreat's
[Battery-Emulator](https://github.com/dalathegreat/Battery-Emulator),
which talks to real Leaf hardware. The implementation is in
[`crates/ev-can`](https://github.com/aphid-ev/aphid-ev/tree/main/crates/ev-can), tested against that frame and
four others.

# Messages

Here are the messages seen on the EV-CAN network. For more ECU-specific
information see the [[inverter|traction inverter]] page.

`crates/ev-can` encodes these messages. It also defines `0x11A` and
`0x50B`, two keep-alives from the VCM, and `0x55A`, the inverter and motor
temperatures, taken from the first VCM prototype. None of the three has
been checked against a real inverter. Bytes marked `???` below are encoded
as the prototype had them and are unverified too.

## `0x1D4` — Torque request

**Sender:** VCM
**Receiver:** Traction inverter
**Frequency:** 100 Hz
**DLC:** 8 bytes

**Data:**

| Byte 0 | Byte 1 | Byte 2 | Byte 3 | Byte 4 | Byte 5 | Byte 6 | Byte 7 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ??? | ??? | Torque MSB | Torque LSB | Seq. counter | ??? | ??? | CRC |

**Torque:** signed `i16`. Positive → accelerate, negative → regen. Scale
*TBD* Nm/bit.

**Sequence counter:** loops `0 → 3`, in bits 7-6 of byte 4.

**CRC:** see above.

## `0x1DA` — Inverter status

**Sender:** Traction inverter
**Receiver:** VCM
**Frequency:** *TBD* Hz
**DLC:** 8 bytes

**Data:**

| Byte 0 | Byte 1 | Byte 2 | Byte 3 | Byte 4 | Byte 5 | Byte 6 | Byte 7 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DC voltage MSB | DC voltage LSB | ??? | ??? | ??? | ??? | ??? | ??? |

**DC voltage:** unsigned `u16`, scale `0.5 V/bit`.

# References

- Reverse engineering by [8Dromeda](http://productions.8dromeda.net/c55-leaf-inverter-protocol.html).
- GitHub repository [leaf_can_bus_messages](https://github.com/dalathegreat/leaf_can_bus_messages).
