---
id: 6duk.wg
title: Re-render the schematics that cross inline connectors after the wirebug fixes
status: open
priority: P1
created: "2026-11-23T09:55:14.369754Z"
updated: "2026-11-23T09:55:14.369754Z"
tags:
  - wiring
---

## Summary

The wiring model uses wirebug 0.2.0, which has two drawing defects reported
upstream:

- [LHelge/wirebug#20](https://github.com/LHelge/wirebug/issues/20): a
  schematic view does not draw a connection that passes through an `inline`
  connector. It draws a stub on each side.
- [LHelge/wirebug#21](https://github.com/LHelge/wirebug/issues/21): titles
  and wires are drawn over neighbouring includes.

Once a wirebug release contains the fixes, update the pinned wirebug version
and check the drawings again.

## Steps

- Update the wirebug version the repository pins and installs in CI.
- Run `just wiring check` and `just wiring render`.
- Check the schematics that cross inline connectors first: "Direction
  selector" (`ic_gear`) and the brake switch drawing (`ic_brake`).
- Check the fuse box views, "VCM I/O", "Front BMS I/O" and "Rear BMS I/O"
  for overlapping titles and wires. Remove layout workarounds that are no
  longer needed, such as the 5-cell spacing in the fuse box views.
- Update the wiki text if a drawing now shows something the page explains
  in words because the drawing could not.

## Acceptance criteria

- The pinned wirebug version contains the fixes for both issues.
- Every connection through an inline connector is drawn end to end in the
  schematic views.
- No title or wire overlaps a neighbouring include in any view.
- `just wiring check` and `just site build` pass.