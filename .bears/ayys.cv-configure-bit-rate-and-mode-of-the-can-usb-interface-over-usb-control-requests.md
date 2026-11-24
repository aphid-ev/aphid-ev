---
id: ayys.cv
title: Configure bit rate and mode of the CAN/USB interface over USB control requests
status: done
priority: P1
created: "2026-11-24T17:15:23.304110103Z"
updated: "2026-11-24T18:28:19.436332382Z"
tags:
  - can-usb
depends_on:
  - ayys.89
  - ayys.dr
attempts: 1
---

## Summary
The bit rate is fixed at 500 kbit/s in `can.rs`, and the host has no way to ask what it's talking to. Implement the vendor control requests that `ayys.rv` defines: device info, set config (bit rate and mode), start and stop the bus. Add the matching host library calls and CLI options in the same commit so the vertical change is atomic.

## Component
`ecus/can-usb/firmware`, `ecus/can-usb/host` (library and CLI). Also: `site/content/wiki/can-usb.md`, `crates/can-usb-protocol` if a type needs adjusting.

## Acceptance criteria
- [ ] Firmware answers vendor requests on the interface through an `embassy_usb::Handler`: GET device info (protocol version, firmware version, serial), SET config (`Config`), start bus, stop bus. Unknown requests and undecodable payloads are STALLed (`Rejected`), never a panic.
- [ ] Set config while the bus runs stops it, applies the config and restarts it. While stopped, STB is high (standby) and no frames are sent; frames arriving from USB in that state are discarded with an error event or a counted drop (pick one, document it).
- [ ] Bit rates 125k/250k/500k/1M and modes Normal, ListenOnly (bus monitoring: no ACK, no TX) and InternalLoopback map to the FDCAN settings. Check that the bit timing embassy computes from the 48 MHz FDCAN clock samples at about 87.5 %. If it doesn't, set the timing explicitly.
- [ ] Power-on default stays 500 kbit/s Normal, running, so hosts that never configure keep working.
- [ ] The CAN side owns reconfiguration in one task (e.g. a `CanControl` channel from the USB handler to the CAN supervisor that stops the RX/TX loops, reconfigures and restarts), not shared mutable peripherals across tasks.
- [ ] Host library: `device_info()`, `set_config()`, `start()`, `stop()`. `open()` reads device info and returns `Error::ProtocolMismatch { device, host }` when the versions differ. The CLI gains `--bitrate` and `--mode` and an `info` command.
- [ ] Manager keeps building (`just host ci`).
- [ ] Wiki: the control requests (code, direction, payload, effect), defaults, and the CLI options.

## Implementation notes
- The state machine (stopped/running, what a request does in each state, which config is valid) belongs in a host-testable type in `crates/` (the protocol crate or a small `can-usb-core`), with the firmware only executing its decisions.
- embassy's FDCAN split: reconfiguring means dropping `Can`/`CanTx`/`CanRx` and building a new `CanConfigurator`. Check what embassy-stm32 0.6 allows (e.g. `Can::into_config`) before designing the task layout.
- The handler runs inside `usb.run()`. Keep it non-blocking: it posts to a channel or signal and answers; for GET it reads a snapshot.

## Edge cases
- Set config with an invalid bit rate value: STALL, current config unchanged.
- Stop while TX frames are queued: they're discarded and counted, not sent after the next start with a different bit rate.
- Host disconnects while stopped: the bus stays stopped until a new host starts it, or returns to defaults on USB reset. Decide which, and document it (returning to defaults on USB reset is the least surprising).

## Verification
- Host tests for the state machine and for encoding/decoding every control payload. (The fake USB device for testing the host library arrives with the host throughput task.) `just host ci`, `just can-usb ci-firmware`.
- Wiki updated.

## Not verifiable here
Correct bit rates on a real bus, ListenOnly not ACKing: the bench task.