---
id: 2h7v
title: "M9: Finalize the BMS firmware"
type: epic
status: open
priority: P1
created: "2026-11-22T10:52:23.602228555Z"
updated: "2026-11-22T10:52:23.602228555Z"
tags:
  - bms
  - crates
  - safety
depends_on:
  - 6duk
  - thmn
---

## Goal
Master firmware that measures every cell, enforces the pack limits, controls its contactor (and precharge on the front BMS) and reports to the VCM.

## Scope
- Behaviour in a `crates/` BMS library with host tests: cell and temperature limits, state of charge, the current-limit calculation, balancing decisions, and the front BMS's precharge sequence.
- `ecus/bms/firmware`: isoSPI driver for the slave front ends, shunt ADC, contactor drivers and CAN2 messages from M1. Creating it also stops `just bms ci-firmware` falling back to the host workspace.
- Linked behind the bootloader from M3.

## Decisions
- One firmware runs on both masters; front or rear is a parameter or node ID from M1.

## Safety
Every limit, the precharge timing and the contactor fault reactions follow M1. Each is covered by a host test.

## Acceptance
- [ ] `just bms ci-firmware` and `just host ci` pass.

## Verified on hardware
On one module, then one pack with current limited: limits trip and open the contactor at the specified values, and precharge completes or aborts as specified.