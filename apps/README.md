# Apps

`manager/` — cross-platform desktop application (egui) for managing the ECUs
over CAN: reading and writing parameters, live data and firmware updates. It
talks to the bus through the CAN/USB interface's host library and shares the
protocol crates in [`crates/`](../crates/) with the firmware.
