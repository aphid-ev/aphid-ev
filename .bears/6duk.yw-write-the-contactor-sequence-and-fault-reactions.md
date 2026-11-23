---
id: 6duk.yw
title: Write the contactor sequence and fault reactions
status: open
priority: P0
created: "2026-11-22T10:53:14.253488727Z"
updated: "2026-11-23T18:32:19.888355Z"
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

## Decided in the wiring review
- Each BMS measures its cell stack and its pack output behind the contactors and reports both on CAN2. The VCM decides when precharge is complete from those values and compares them with the inverter's bus voltage from CAN1. Write the limits and the reaction to a disagreement.
- Welded contactors are detected from the BMS voltage sense; the auxiliary contacts are not wired. The output sense spans both main contactors of a pack, so one welded contactor only shows when the other is closed alone. Write that check into the sequence.
- Precharge, the MSD and the current shunt are in the front pack.
- A BMS sleeps until traffic on CAN2 wakes it through its transceiver, then holds itself on from its always-on feed. The VCM can therefore read BMS data before it enables safe power. Write how long the VCM waits for both BMS controllers to report after it starts talking, and when a BMS goes back to sleep.
- Safe power is a monitoring input on each BMS. Write what a BMS does when safe power drops while its contactors are closed, and when it is present without a close command.

## Expected result
A specification the CAN2 message crate, M6 and M9 can be tested against.