---
id: ayys.89
title: Keep received and sent frames in order and report lost frames on the CAN/USB interface
status: open
priority: P1
created: "2026-11-24T17:14:56.123236887Z"
updated: "2026-11-24T17:14:56.123236887Z"
tags:
  - can-usb
depends_on:
  - ayys.rv
---

## Summary
The firmware's data path has three correctness problems. Received frames pass through a priority channel and reach the host sorted by identifier instead of in bus order. When the three FDCAN TX buffers are full, the driver evicts the lowest-priority frame (it is logged and never sent). And the filter only accepts standard identifiers. Replace the channels with FIFOs, make transmission push back instead of dropping, accept every identifier, and turn receive-side loss into an `Overrun` event.

## Component
`ecus/can-usb/firmware`. Also: `site/content/wiki/can-usb.md` (CAN side section). Counting and coalescing logic that grows beyond a few lines goes in `crates/can-usb-protocol` (or a small new crate) with host tests.

## Acceptance criteria
- [ ] RX (CAN→USB) and TX (USB→CAN) queues are `embassy_sync::channel::Channel` FIFOs, sized for at least 20 ms of a saturated 500 kbit/s bus in the RX direction (≈ 100 events) and a few full USB packets in the TX direction. State the RAM cost in a comment.
- [ ] TX: FDCAN TX buffers run in FIFO (queue) mode, not priority mode, and the TX task waits for a free buffer instead of evicting. The USB read task blocks on a full TX queue, so the host sees NAKs (back-pressure) and no frame is dropped. If embassy-stm32 0.6 cannot do FIFO mode without eviction, say so in the task and pick the least bad option with the user; do not work around it by patching embassy.
- [ ] RX: when the RX queue is full, the CAN task does not block (blocking lets the 3-element hardware FIFO overflow invisibly). It counts lost frames and, as soon as there is room, enqueues one `Overrun { lost }` with the count, then resets it. FDCAN's own RX FIFO overflow (message lost flag) is counted too where the driver exposes it.
- [ ] Filters accept all standard and all extended identifiers into the RX FIFO.
- [ ] Per-frame defmt logging is `trace`, and `DEFMT_LOG` in `.cargo/config.toml` defaults to `info`, so release builds don't format a log line per frame.
- [ ] `just can-usb ci-firmware` passes, and the wiki's CAN side section describes FIFO order, back-pressure and overrun reporting instead of priority queues and eviction.

## Implementation notes
- Channel types and statics are in `main.rs`. Rename the aliases to say what they carry (`EventChannel`, `FrameChannel`) while you're there.
- Overrun counting: `try_send` in `rx_task`, a saturating `u16`, and a pending flag that is retried before each new frame. A small `OverrunCounter` type with host tests is the cleanest place for the logic.
- While the host isn't connected, `write_ep.write` fails fast. Events are then discarded at the USB end rather than counted as overruns. Keep that behaviour and document it.
- No `unwrap` in the data path.

## Edge cases
- Host stops reading (application paused): RX fills, overruns are counted, and the first event after it resumes is the `Overrun`.
- Host floods TX on a bus with no other node (no ACK): FDCAN retries forever in normal mode, TX blocks, USB NAKs. That is correct behaviour. The host library must handle a send that doesn't complete (covered in the host throughput task).
- Remote frames: say on the wiki whether they're forwarded (the protocol `Frame` has no RTR flag; dropping them with a trace log is acceptable).

## Verification
- Host tests for any extracted logic. `just can-usb ci-firmware`, plus `just host ci` if a crate changed.
- Wiki CAN side section updated.

## Not verifiable here
Order and zero loss under full load: the bench task.