---
name: bears-planning
description: "Plan work in the Aphid EV monorepo as Bears epics and tasks. Use when: breaking a feature into tasks across firmware, shared crates, hardware, the manager app, wiring and the docs site; scoping a new ECU feature, CAN message, parameter or board revision; organising work with dependencies, priorities and manual hardware steps."
---

# Planning work in Aphid EV

A plan here is a Bears epic with dependency-linked tasks in `.bears/`. Most
features cut through several components (a shared crate, one or more
firmwares, the manager app, the documentation), and part of the work can only
be done by the user with KiCad, a debug probe or the car. A good plan makes
both visible: which component each task belongs to, and which tasks an agent
can finish alone.

Planning writes no code and changes nothing outside `.bears/`.

## Tools

Use the Bears MCP tools (`mcp__bears__*`) when they are loaded, otherwise the
`bea` CLI with `--json`; see [the CLI reference](./references/cli-fallback.md).
Task files are Markdown with YAML frontmatter at `.bears/<id>-<slug>.md`:
create a task with the tool, then write its body by editing the file.

## 1. Understand the request

- Ask the user about scope before researching: what the feature must do, on
  which units, what is explicitly not part of it, and whether hardware exists
  to test it on. Ask again when research turns up a decision that is theirs.
- Read the root `AGENTS.md` and the scoped `AGENTS.md` of every component the
  work touches, the wiki pages in `site/` that describe the area, and the code
  next to where the change goes.
- Check `bea list --json` and `bea epics --json` for work that already covers
  part of it.
- Settle these before writing tasks, because they shape the breakdown:
  - **Which wire formats change?** CAN messages, parameters, the bootloader
    protocol, the flash layout. Is the change compatible with firmware and
    bootloaders already flashed? Bootloaders in the field cannot be updated
    over CAN.
  - **Is it safety-relevant?** Anything touching contactor control, precharge,
    current or voltage limits, or fault handling in the VCM or BMS. The
    required behaviour, including the fault behaviour, must be written down in
    the epic by the user, not chosen by an implementer.
  - **Does it need a board change?** A new pin, part or connector means a
    KiCad task for the user, and firmware that states which board revisions it
    supports.
  - **Does the wiring change?** A new signal between units belongs in
    `wiring/` as well.

## 2. Design the breakdown

One task is one component and one concern, small enough for a single focused
session and a single review.

**Order from the shared definition outwards.** A vertical feature usually
follows this chain, each step depending on the one before:

1. the shared crate in `crates/`: message, parameter or protocol definition
   and the logic that goes with it, with host tests;
2. the bootloader, when the protocol or flash layout is involved;
3. each firmware that produces or consumes it, one task per ECU;
4. the CAN/USB host library, when the transport changes;
5. the manager's core library and CLI, then its UI;
6. wiring and hardware exports, when signals or boards change;
7. hardware verification, as `manual` tasks.

A change that breaks a wire format cannot be split so that `main` is broken
between tasks: the task that changes the shared crate also updates every user
enough to keep each one building, and later tasks add the behaviour.

**Tests and documentation belong to the task that changes the behaviour.**
Each task includes its host tests and the update of the wiki page it makes
untrue. Give documentation its own task only for a new page or a restructure.
Blog posts are the user's build log: do not plan one unless they ask, and then
as a `manual` task.

**Put behaviour where it can be tested.** If a task would add logic to a
firmware binary, plan it as logic in a `crates/` library with host tests plus
a thin firmware task that wires it to peripherals.

**Separate what an agent can do from what it cannot.** Tag a task `manual`
when only the user can do it:

- KiCad schematic and layout changes, footprint and library work;
- flashing, and anything checked with a probe, a scope, a bench supply or the
  car;
- ordering boards or parts;
- tagging a release.

A firmware task ends when it builds and its logic passes host tests. Follow it
with a `manual` task for the bench check, depending on the firmware task, that
lists exactly what to observe. Do not make agent tasks depend on a bench check
unless the result really decides what is built next.

## 3. Create the epic

The epic's body is what an implementer reads to understand the purpose and
what a reviewer holds the whole branch against.

```markdown
## Goal
What the car or the tooling can do when this is finished, in the user's terms.

## Scope
Components touched, and what is deliberately left out.

## Decisions
Every choice already made with the user: message IDs and layouts, parameter
names and units, limits and timings, fault behaviour, compatibility with
flashed firmware, supported board revisions.

## Safety
What must hold in every state, and the defined fault behaviour. "None" when
the epic touches nothing safety-relevant.

## Acceptance
- [ ] Criteria for the epic as a whole.

## Verified on hardware
What the user checks on the bench or in the car before this counts as done.
```

## 4. Create the tasks

Create them in dependency order, each with `parent` set to the epic and
`depends_on` set to the tasks it builds on. Dependencies are execution order
for `implement-epic`, so add one only where the later task needs the earlier
one's result.

**Tags.** One component tag, plus `manual` and `safety` where they apply:

| Tag | For |
| --- | --- |
| `crates` | shared crates |
| `bootloader`, `can-usb`, `vcm`, `bms` | that unit; add `hw` for its KiCad project |
| `manager` | desktop app, core library and CLI |
| `wiring` | wirebug model |
| `site` | documentation |
| `ci` | justfiles and workflows |
| `manual` | only the user can do it |
| `safety` | changes safety-relevant behaviour |

**Priorities.** P0 blocks other work or fixes something broken; P1 is the
epic's critical path; P2 is standard work in the epic; P3 is polish and
follow-ups.

**Task body.** Enough that an implementer can start without asking:

```markdown
## Summary
What this task does and why, in one paragraph.

## Component
The directory it works in, and any other path it is allowed to touch.

## Acceptance criteria
- [ ] Specific and testable. Each is implemented and covered by a test, or
      verified by a named command.

## Implementation notes
- Files and modules to change, patterns to follow from nearby code.
- Types, messages, parameters and recipes involved, by name.
- Decisions from the epic that bind this task.

## Edge cases
- Error and fault paths, invalid input, timeouts, bus-off, a missing or
  outdated peer.
- Compatibility with firmware or bootloaders already flashed.

## Verification
- Host tests to add.
- The `just` recipes that must pass, from the table in the root `AGENTS.md`.
- Wiki pages to update.

## Not verifiable here
What only a real board can show, and the `manual` task that covers it.
```

For a `manual` task, replace the last three sections with **Steps** (what to
do, with the exact recipe or tool) and **Expected result** (what to observe or
measure, with numbers where there are any).

## 5. Check and present the plan

- `bea list --epic <id> -a --json` shows every task; `bea ready --epic <id>`
  shows only the ones with nothing before them; `bea graph` shows the order.
- Walk the chain once: after each task, does every workspace still build?
- Present a table to the user and ask for corrections:

```
Epic: <title> (<id>)

| # | Task | ID | Component | Priority | Depends on | Who |
|---|------|----|-----------|----------|------------|-----|
| 1 | Define the message in the protocol crate | a1b.c2 | crates | P1 | — | agent |
| 2 | Send it from the BMS | a1b.d4 | bms | P1 | a1b.c2 | agent |
| 3 | Check the value on the bench | a1b.e7 | bms | P2 | a1b.d4 | manual |
```

Say which decisions you recorded in the epic, and which questions are still
open.
