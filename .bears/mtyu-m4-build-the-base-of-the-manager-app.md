---
id: mtyu
title: "M4: Build the base of the manager app"
type: epic
status: open
priority: P1
created: "2026-11-22T10:51:40.513175370Z"
updated: "2026-11-22T10:51:52.597461629Z"
tags:
  - manager
depends_on:
  - 6duk
  - ayys
  - thmn
---

## Goal
A desktop app that finds the dongle, shows what is on the bus and which ECUs are present, and updates their firmware. Later epics add per-ECU pages to it.

## Scope
- `apps/manager`: a UI-free core library with a CLI, and an egui front end. Adding it brings back the `apps/*` workspace glob.
- The CAN transport behind a trait, with `can-usb-host` as one implementation and a fake bus for tests.
- A bus view, ECU discovery, and reading and writing parameters with the protocol from M1.
- Firmware update in the GUI, on top of the flashing core from M3.
- A structure where each ECU's configuration and diagnostics plug in as a page.

## Decisions
- Never open a real interface or write to an ECU unless the user starts it.

## Safety
None directly. A firmware update can leave an ECU without an application, so the flow stays resumable and never skips verification.

## Acceptance
- [ ] `just host ci` covers the core library against the fake bus.
- [ ] Builds on Linux, Windows and macOS.

## Verified on hardware
- Discover a node running `can-echo`, update it from the GUI, and see the new version reported.