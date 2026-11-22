# CAN/USB interface guidelines

In addition to `ecus/AGENTS.md`.

- The USB wire format is `crates/can-usb-protocol`. The firmware and the host
  library are installed separately, so a change there is breaking: update
  both sides in the same commit and the pinned-bytes tests only on purpose.
- The firmware decodes each bulk transfer with `postcard::take_from_bytes`
  in a loop, because one transfer may carry several frames. `from_bytes`
  would drop everything after the first.
- Bulk OUT `0x01` and bulk IN `0x82`, and VID/PID `0xc0de:0xf055`, are fixed:
  the host opens them by value.
- Never change `DEVICE_INTERFACE_GUID` in `firmware/src/usb.rs`. Windows
  caches the WinUSB binding against it.
- This unit runs from `0x08000000` with no CAN bootloader; it is reachable
  over USB. Its `memory.x` comes from embassy's `memory-x` feature, which an
  ECU application must not use.
