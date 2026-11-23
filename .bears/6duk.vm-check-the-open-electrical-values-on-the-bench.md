---
id: 6duk.vm
title: Check the open electrical values on the bench
status: open
priority: P2
created: "2026-11-23T18:12:37.848590Z"
updated: "2026-11-23T18:12:37.848590Z"
tags:
  - wiring
  - site
  - manual
---

## Summary
The wiring review left values that only a measurement or a look at the hardware can settle. Each is marked in the model or on the wiki as a value to check.

## To check
- Which of pins 3 and 8 on the motor's resolver connector is EX+. The source names both only as the excitation coil; `motor.wb` has EX+ on pin 3.
- The current the OBC/DC-DC draws on IGCT, against the 7.5 A EV-signal fuse, and the current on its sense input while the car sleeps.
- The total load of the Beetle's original 12 V system, against the 60 A MIDI fuse and 10 mm² cable.
- The real load of the rear EV fuse box. Its fuses add up to 63 A against a 60 A MIDI fuse on 10 mm²; 80 A on 16 mm² would use the full stud rating.
- The lamp wattages, against the 15 A VCM fuse that assumes three 21 W stop lamps and one 21 W reverse lamp.
- The wire colours on the OBC/DC-DC signal tail, against the pin table on the `obc` page.

## Acceptance criteria
- [ ] Each value is measured and the model and the wiki carry the result.
- [ ] `just wiring check` passes and the site builds.