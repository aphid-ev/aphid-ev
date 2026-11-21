---
id: u4mx
title: Make the host workspace load before apps/manager exists
status: open
priority: P2
created: "2026-11-21T12:51:14.878575385Z"
updated: "2026-11-21T12:51:14.878575385Z"
tags:
  - host
  - ci
---

The root `Cargo.toml` lists `apps/manager` as a workspace member, but `apps/manager/Cargo.toml` does not exist yet. Every Rust job (`just host ci`, the ECU firmware jobs, the bootloader) fails at `cargo metadata` with "failed to load manifest for workspace member apps/manager". Seen on PR #1, whose changes to the root justfile and setup action trigger those workflows.

Done when `just host ci`, `just bootloader ci` and the `ci-firmware` recipes pass on the scaffold: either drop members that do not exist yet, or add minimal crates for them.