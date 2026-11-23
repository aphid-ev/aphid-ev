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
The bootloader in `ecus/bootloader` uses it, and so does the programming
device's side in `apps/manager`. The identifiers and error codes below are copied from that crate by
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

**Erase flash** works in pages of 2 KiB on both chips: 64 pages on the
STM32G431 and 256 on the STM32G491. The bootloader takes the first four.

An application must be linked at `0x08002000`. When it starts the
application, the bootloader points the vector table register at the start
of the application region and jumps to the reset vector it finds there. An
image linked at `0x08000000` is still written to the application region,
but its vectors point into the bootloader. The `can-echo` test application
in `ecus/bootloader/can-echo` shows the linker setup.

The bootloader does not reset the clocks before the jump. The application
starts with the system clock at 144 MHz from the 16 MHz crystal and the
FDCAN clock at 48 MHz. The flash wait states and the PWR and SYSCFG clocks
also stay as the bootloader set them.

The peripherals the bootloader used are reset before the jump. FDCAN,
GPIOB and the CRC unit go through their RCC reset and have their clocks
stopped, and SysTick is cleared to its reset values.

# Image header {#image-header}

Every application carries a 16-byte header at offset `0x200` in its image,
right after the vector table. The bootloader checks the image against it
before it starts the application. The fields are little-endian.

| Offset | Size | Field | Value |
| --- | --- | --- | --- |
| `0x200` | 4 | Magic | `AEVI` in ASCII, `0x49564541` as a word |
| `0x204` | 4 | Length | Bytes from the start of the image to its end, a multiple of 4 |
| `0x208` | 4 | CRC | CRC-32 over the first `length` bytes of the image, without these 4 |
| `0x20c` | 3 | Version | Major, minor and patch |
| `0x20f` | 1 | Reserved | `0xff` |

The CRC is CRC-32/ISO-HDLC, the CRC-32 of zlib. Its check value, the CRC
of the ASCII string `123456789`, is `0xcbf43926`. The bootloader computes
it with the CRC unit of the STM32G4.

The image is valid when the magic matches, the length is at least `0x210`
and fits in the application region, and the CRC matches. Without a valid
image the node stays in the bootloader. It does not start the application
when the wakeup window closes, and it refuses **Start application**.

An update that stops partway leaves part of the image erased, so its CRC
does not match. **Erase flash** removes the header with the first page,
so the image is invalid from the moment an update starts.

The application reserves the header with `boot_protocol::image_header!()`,
which takes the version from the application's `Cargo.toml`. Its
`memory.x` places the `.image_header` section at `0x200` and fills the
gap after the vector table with `0xff`. `ecus/bootloader/can-echo` shows
the setup. After linking, the image tool `tools/boot-image` fills in the
length and the CRC. It writes the sealed header back into the ELF, for
flashing with a debug probe, and writes the raw image for flashing over
CAN. An image flashed without the tool still holds `0xffffffff` in both
fields, and the bootloader rejects it.

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

1. The node checks its image, sends **Wakeup** with the result, then
   waits 500 ms.
2. The programming device answers with **Catch bootloader** inside that
   window. Otherwise the node starts its application if the image is
   valid, and stays in the bootloader if it is not. A node that stays in
   the bootloader takes commands without a catch.
3. The programming device sends **Erase flash** and waits for the
   **Command response**.
4. For each block, the programming device sends **Program block**, then
   the **Block data** frames that cover the block, then waits for the
   **Command response**.
   The image is sealed before it is sent, so the block that holds the
   header is a block like any other, and the blocks may go in any order.
5. The programming device sends **Start application**. The node checks
   the image again and starts it, or answers with error 7.

## Finding the node {#probe}

The programming device does not know when the node resets, and a node
left in the bootloader by an interrupted update sends no wakeup frame. To
find the node in both cases, the programming device sends a probe every
100 ms: an **Erase flash** with the wrong key. A node in the bootloader
refuses it with error 4 and erases nothing. A node in its wakeup window
or its application ignores it.

When a **Wakeup** from the node arrives, the programming device answers
with **Catch bootloader** at once. The next probe then gets error 4, and
the node is ready for **Erase flash**. Running an interrupted update again
therefore finds the node in its bootloader and starts over.

## Programming tool {#tool}

The manager's command line flashes a sealed image into a node through
the [[can-usb]]:

```sh
cargo run -p manager -- flash --node 0 can-echo.bin
```

It checks the image before it sends anything, waits up to 30 seconds for
the node, and skips blocks that are all `0xff`. For `can-echo`,
`just bootloader flash-echo-can` builds, seals and flashes it in one step.

# Frames

| Identifier | Frame | Sent by | DLC |
| --- | --- | --- | --- |
| `0x7ff` | Wakeup | Node | 5 |
| `0x7f0` + node | Catch bootloader | Programming device | 0 |
| `0x7ef` | Command response | Node | 0 or 1 |
| `0x7e0` + node | Bootloader command | Programming device | 0, 4 or 8 |
| `0x7d0` + node | Block data | Programming device | 2 to 8 |

## `0x7ff` wakeup

The node sends this frame once when it starts. It then waits 500 ms for
**Catch bootloader** before it starts the application.

| Byte 0 | Byte 1 | Byte 2 | Byte 3 | Byte 4 |
| --- | --- | --- | --- | --- |
| `node_id` | Version major | Version minor | Version patch | Image status |

The version comes from the image header. It reads 0.0.0 when the image is
not valid. The image status says why:

| Status | Name | Meaning |
| --- | --- | --- |
| 0 | Valid | The image passed the check, and the node starts it when the window closes |
| 1 | Missing | No header magic. The region is erased, or the image was built without a header |
| 2 | Length | The length is not sealed, is shorter than `0x210`, does not fit in the region or is not a multiple of 4 |
| 3 | Checksum | The image does not match its CRC, usually after an interrupted update |

## `0x7f0 + node` catch bootloader

An empty frame. A node that receives it during the wakeup window stays in
the bootloader and waits for commands. A catch frame with a payload is
ignored.

## `0x7ef` command response

The node answers every bootloader command addressed to it with one
response, including the commands it refuses, with two exceptions:

- **Start application** gets no response when the image is valid, because
  the node starts the application at once. It gets error 7 when the image
  is not valid.
- A command frame with a DLC other than 0, 4 or 8 is ignored.

**Catch bootloader** and **Block data** are not commands and get no
response of their own. The response to **Program block** comes after its
block data, unless the node refuses the block as soon as the command
arrives.

The identifier carries no node ID, because `0x7ef` plus a node ID would
collide with the catch frames at `0x7f0`.

A node drops a response or wakeup frame that it cannot queue within
100 ms. That happens when no other node acknowledges its frames, or when
higher-priority traffic fills the bus for that long. A programming device
that gets no response within its own timeout starts the update again from
**Erase flash**. Sending **Program block** again is not enough, because
flash that the lost block already wrote cannot be written twice without
an erase.

An empty frame means the command succeeded. A one-byte frame carries an
error code:

| Code | Name | Meaning |
| --- | --- | --- |
| 0 | Overflow | Block data ran past the length in **Program block**, or the block is empty, longer than 1792 bytes or does not fit in the application region |
| 1 | Checksum | The assembled block did not match the CRC in **Program block** |
| 2 | Header | The node could not build a response frame |
| 3 | FdCan | The CAN peripheral rejected a transmission |
| 4 | InvalidKey | **Erase flash** carried the wrong key |
| 5 | Alignment | The block offset is not a multiple of 8 |
| 6 | Program | The flash peripheral reported an erase or programming failure |
| 7 | InvalidImage | **Start application** found no valid image |
| 8 | Timeout | No block data arrived for 500 ms while the node collected a block |

## `0x7e0 + node` bootloader command

The DLC tells the commands apart. A command frame with any other DLC is
ignored and gets no response.

### Start application

`dlc = 0`. The node checks the image again. With a valid image it starts
the application and sends no response. Otherwise it answers with error 7
and stays in the bootloader.

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
multiple of 8, the programming granularity of the flash.

`len` is the number of bytes in the block, from 1 to 1792. That is at most
256 sub-blocks of 7 bytes. The node uses `len` to tell when the block is
complete.

The node checks the block against the application region as soon as this
frame arrives. An offset that is not a multiple of 8 gets error 5. A
block that is empty, longer than 1792 bytes or runs past the end of the
region gets error 0. A refused block writes nothing, and the node ignores
the block data that follows it.

`crc16` is CRC-16/IBM-SDLC over the `len` bytes of the block. Its check
value, the CRC of the ASCII string `123456789`, is `0x906e`. The node
checks it before it writes anything to flash.

When `len` is not a multiple of 8, the node pads the last double word with
`0xff`, which matches erased flash.

While the node collects a block it ignores every frame except block data
for itself, until a sub-block reaches `len`. If no block data for the node
arrives for 500 ms, the node drops the block, writes nothing, answers
with error 8 and waits for commands again. The 500 ms count from the last
sub-block, so a long block that arrives without gaps never times out.

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

- Every ECU's bootloader is built with node ID 0, so only one ECU can be
  on the bus while it is flashed.
- Only `can-echo` links at the application offset and carries an image
  header. The ECU firmware still links at `0x08000000` and is flashed with
  a debug probe.
