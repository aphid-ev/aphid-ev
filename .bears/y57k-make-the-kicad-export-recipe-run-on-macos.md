---
id: y57k
title: Make the KiCad export recipe run on macOS
status: open
priority: P3
created: "2026-11-23T18:12:37.879190Z"
updated: "2026-11-23T18:12:37.879190Z"
tags:
  - ci
---

## Summary
`just docs` fails on macOS in the KiCad `export` recipe with `shopt: globstar: invalid shell option name`. macOS ships bash 3.2, which has no `globstar`. The wiring render and the site build work on their own, so the failure only blocks the combined recipe.

## Acceptance criteria
- [ ] `just docs` runs to the end on macOS and on the CI runner.

## Implementation notes
- Replace the `**` glob with `find`, or require a newer bash explicitly in the recipe's shebang.