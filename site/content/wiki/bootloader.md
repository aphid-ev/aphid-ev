---
title: CAN bootloader protocol
category: Software
tags: [canbus, software, bootloader]
created: 2026-11-22
---

Every DIY ECU starts in a bootloader that sits in the first 8 KiB of its
flash. The bootloader lets a programming device replace the ECU's
application over CAN, through the [[can-usb]], so a
firmware update does not need a debug probe. This page specifies the
protocol between the bootloader and the programming device.

The protocol is implemented once, in
[`crates/boot-protocol`](https://github.com/aphid-ev/aphid-ev/tree/main/crates/boot-protocol).
The bootloader in `ecus/bootloader` uses it, and so will the programming
tool. The identifiers and error codes below are copied from that crate by
hand, so check the crate when the two disagree.

A bootloader that is already in an ECU can only be replaced with a debug
probe. The protocol is therefore frozen. New frames may be added, but an
existing frame never changes meaning.

# Bus

The protocol uses classic CAN frames with standard 11-bit identifiers at
500 kbit/s, sampled at 87.5 %. Multi-byte values are big-endian.

# Flash layout {#flash-layout}

| Region | Start | Size on STM32G431 | Size on STM32G491 |
| --- | --- | --- | --- |
| Bootloader | `0x08000000` | 8 KiB | 8 KiB |
| Application | `0x08002000` | 120 KiB | 504 KiB |

An application must be linked at `0x08002000`. When it starts the
application, the bootloader points the vector table register at the start
of the application region and jumps to the reset vector it finds there. An
image linked at `0x08000000` is still written to the application region,
but its vectors point into the bootloader. The `can-echo` test application
in `ecus/bootloader/can-echo` shows the linker setup.

The bootloader does not reset the clocks before the jump. The application
starts with the system clock at 144 MHz from the 16 MHz crystal and the
FDCAN clock at 48 MHz.

# Node IDs {#node-ids}

Each node has an ID from 0 to 15. The per-node identifiers below add the
node ID to a base, so the frames for node 3 at base `0x7f0` use `0x7f3`.

| Node ID | Role |
| --- | --- |
| 0 | Bus master |
| 1 to 14 | Other nodes |
| 15 | Programming device |

The programming device never runs a bootloader. Its own catch identifier
`0x7ff` is the wakeup identifier, and its command identifier `0x7ef` is
the response identifier. That overlap is harmless, and a node with a
bootloader must never use ID 15.

# Programming sequence

1. The node sends **Wakeup** when it starts, then waits 500 ms.
2. The programming device answers with **Catch bootloader** inside that
   window. Otherwise the node starts its application.
3. The programming device sends **Erase flash** and waits for the
   **Command response**.
4. For each block, the programming device sends **Program block**, then
   the **Block data** frames that cover the block, then waits for the
   **Command response**.
5. The programming device sends **Start application**.

# Frames

| Identifier | Frame | Sent by | DLC |
| --- | --- | --- | --- |
| `0x7ff` | Wakeup | Node | 4 |
| `0x7f0` + node | Catch bootloader | Programming device | 0 |
| `0x7ef` | Command response | Node | 0 or 1 |
| `0x7e0` + node | Bootloader command | Programming device | 0, 4 or 8 |
| `0x7d0` + node | Block data | Programming device | 2 to 8 |

## `0x7ff` wakeup

The node sends this frame once when it starts. It then waits 500 ms for
**Catch bootloader** before it starts the application.

| Byte 0 | Byte 1 | Byte 2 | Byte 3 |
| --- | --- | --- | --- |
| `node_id` | Version major | Version minor | Version patch |

## `0x7f0 + node` catch bootloader

An empty frame. A node that receives it during the wakeup window stays in
the bootloader and waits for commands. A catch frame with a payload is
ignored.

## `0x7ef` command response

The node sends one response for every bootloader command addressed to it,
including commands it refuses. The identifier carries no node ID, because
`0x7ef` plus a node ID would collide with the catch frames at `0x7f0`.

An empty frame means the command succeeded. A one-byte frame carries an
error code:

| Code | Name | Meaning |
| --- | --- | --- |
| 0 | Overflow | Block data ran past the length in **Program block**, or that length exceeds 1792 |
| 1 | Checksum | The assembled block did not match the CRC in **Program block** |
| 2 | Header | The node could not build a response frame |
| 3 | FdCan | The CAN peripheral rejected a transmission |
| 4 | InvalidKey | **Erase flash** carried the wrong key |
| 5 | Alignment | The block offset is not a multiple of 8 |
| 6 | Program | The flash peripheral reported an erase or programming failure |

## `0x7e0 + node` bootloader command

The DLC tells the commands apart. A command frame with any other DLC is
ignored and gets no response.

### Start application

`dlc = 0`. The node starts the application.

### Erase flash

`dlc = 4`.

| Bytes 0 to 3 |
| --- |
| `key: u32` |

The node erases every page of the application region, then responds. A
key other than `0x12345678` gets error 4 and erases nothing.

### Program block

`dlc = 8`.

| Bytes 0 to 3 | Bytes 4 and 5 | Bytes 6 and 7 |
| --- | --- | --- |
| `offset: u32` | `len: u16` | `crc16: u16` |

`offset` counts from the start of the application region. It must be a
multiple of 8, the programming granularity of the flash. The node checks
the offset when it writes the block, so error 5 arrives after the block
data, not after this frame.

`len` is the number of bytes in the block, at most 1792. That is 256
sub-blocks of 7 bytes. The node uses `len` to tell when the block is
complete.

`crc16` is CRC-16/IBM-SDLC over the `len` bytes of the block. Its check
value, the CRC of the ASCII string `123456789`, is `0x906e`. The node
checks it before it writes anything to flash.

When `len` is not a multiple of 8, the node pads the last double word with
`0xff`, which matches erased flash.

While the node collects a block it ignores every frame except block data
for itself. It cannot be interrupted until a sub-block reaches `len`.

## `0x7d0 + node` block data

`dlc = 1 + N` for a frame that carries N bytes, with N from 1 to 7.

| Byte 0 | Bytes 1 to N |
| --- | --- |
| `index` | Data |

The frame carries bytes `index * 7` up to `index * 7 + N` of the block.
The node places each sub-block by its index, not by the order it arrives
in.

The node treats the block as complete when a sub-block ends exactly at
`len`. Send the sub-blocks in order. If the final one arrives early, the
block is closed with holes in it, and the CRC check rejects it.

# Current limitations {#limitations}

- No host tool sends the programming device's side of the protocol yet.
  The `can-usb` command line tool only listens and sends test frames.
- Every ECU's bootloader is built with node ID 0, so only one ECU can be
  on the bus while it is flashed.
- The wakeup frame always reports version 1.0.0. The bootloader does not
  yet read a version from the application.
- The bootloader does not check that the application region holds a valid
  image before it jumps there.
