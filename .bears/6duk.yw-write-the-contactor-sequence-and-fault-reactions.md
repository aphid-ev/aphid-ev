---
id: 6duk.yw
title: Write the contactor sequence and fault reactions
status: open
priority: P0
created: "2026-11-22T10:53:14.253488727Z"
updated: "2026-11-22T10:53:14.253488727Z"
tags:
  - manual
  - site
  - safety
---

## Summary
The VCM monitor (high side) and both BMS controllers (low side, front also precharge) share the contactor coils. Their firmware needs one written specification of the sequence and of what every unit does on every fault. The user writes it; agents implement it.

## Steps
- On the `contactors` and `safety` wiki pages, write: the close sequence with precharge timing and the voltage match that ends it; the open sequence; and the reaction of each unit to a lost CAN2 peer, an open interlock, a monitor/main disagreement, a cell or current limit, and a failed precharge.
- Give every timeout and limit a number.

## Expected result
A specification the CAN2 message crate, M6 and M9 can be tested against.