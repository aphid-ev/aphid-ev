---
id: thmn.my
title: Give the bootloader timeouts while it collects a block and sends a response
status: open
priority: P2
created: "2026-11-23T19:59:32.415509657Z"
updated: "2026-11-23T19:59:32.415509657Z"
tags:
  - bootloader
  - crates
---

Found in the review of the bootloader for thmn.am.

- `state_write` in `ecus/bootloader/src/main.rs` waits for block data with no timeout. If the programming device stops partway through a block, the node ignores every other frame until it is power-cycled.
- `Can::send` wraps `transmit` in `nb::block!`. With no other node on the bus to acknowledge, the TX buffers stay full and a later response blocks forever.

Neither bricks the node: after a power cycle the wakeup window works and an invalid image keeps it in the bootloader. Both make a failed update harder to recover from the programming device alone.

## Acceptance
- An incomplete block is abandoned after a timeout defined in `crates/boot-protocol` and documented on the `bootloader` wiki page. The node answers with an error code and returns to waiting for commands.
- A response that cannot be queued within a bounded time is dropped instead of blocking.
- The timeout logic is host-tested in `crates/boot-protocol`.
- `just bootloader ci` passes and the bootloader stays under 8 KiB.