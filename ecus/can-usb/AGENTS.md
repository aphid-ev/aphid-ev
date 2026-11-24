# CAN/USB interface guidelines

In addition to `ecus/AGENTS.md`.

- The USB protocol is `crates/can-usb-protocol`: the VID/PID, endpoints,
  packet size, control request codes, `PROTOCOL_VERSION` and every type on
  the wire. Never write one of these values as a literal in the firmware or
  the host library. The firmware and the host library are installed
  separately, so a change there is breaking: update both sides in the same
  commit, raise `PROTOCOL_VERSION`, and change the pinned-bytes tests only on
  purpose.
- Build and read bulk packets with the crate's `Packer` and `unpack`. A
  packet holds several values back to back, so `postcard::from_bytes` would
  drop everything after the first.
- Never change `DEVICE_INTERFACE_GUID` in `firmware/src/usb.rs`. Windows
  caches the WinUSB binding against it.
- This unit runs from `0x08000000` with no CAN bootloader; it is reachable
  over USB. Its `memory.x` comes from embassy's `memory-x` feature, which an
  ECU application must not use.
