# App guidelines

The manager is an egui/eframe desktop app in the host workspace.

- Keep the UI thin. Bus access, parameter handling and the firmware-update
  procedure live in a UI-free core library with a CLI front end, so they can
  be tested and scripted without a window.
- Use the shared crates in `crates/` for every message, parameter and protocol
  definition. Do not redefine IDs or layouts here.
- The core must be testable without hardware: put the CAN transport behind a
  trait and test against a fake bus.
- A firmware update can leave an ECU without a valid application. Keep the
  update flow resumable and never skip verification steps to simplify the UI.
- Never open a real CAN interface or write to an ECU unless asked to.
- Verify with `just host ci`.
