---
id: tney
title: Make the direction controller follow the wiki's shift rule
status: open
priority: P1
created: "2026-11-22T10:19:37.858177835Z"
updated: "2026-11-22T10:19:37.858177835Z"
tags:
  - vcm
  - crates
  - safety
---

The wiki's `direction` page says the VCM accepts a direction change only while the car is stopped with the brake applied. `vcm_core::direction::DirectionController` allows a change below 3 m/s and ignores the brake. Decide the rule (stop threshold, brake required), make the controller and its tests follow it, and update the page if the rule changes.