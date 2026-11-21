# Aphid EV — repository guidelines

Monorepo for the electric conversion of a classic VW Beetle. It holds the
documentation site, the vehicle wiring model, ECU hardware (KiCad) and
firmware (Rust, Arm Cortex-M), shared Rust crates and a desktop management
app. Subdirectories have their own `AGENTS.md` with local rules; read the one
for the area you are changing.

## Map

- `site/` — aphid site published at bladlus.se. The single source of truth for documentation.
- `wiring/` — wirebug model of the vehicle's electrical system.
- `crates/` — shared `no_std` crates (CAN protocol, parameters, bootloader protocol). Members of the host workspace.
- `ecus/<name>/hardware/` — KiCad projects. `ecus/<name>/firmware/` — a standalone cargo workspace per firmware.
- `ecus/bootloader/` — bootloader shared by all ECUs, plain Rust without embassy.
- `ecus/can-usb/host/` — host-side USB library for the CAN/USB interface (host workspace member).
- `apps/manager/` — desktop app (egui) for parameters and firmware updates over CAN.
- `tools/` — host CLIs used by the build.

aphid and wirebug live in their own repositories. Do not vendor or patch them
here. If one of them lacks a capability, say so instead of working around it.

## Commands

Use `just` for everything; run `just` to list recipes. CI runs the same
recipes, so a change that passes `just <component> ci` locally passes CI.

- `just host ci` — fmt, clippy and tests for the host workspace.
- `just <ecu> build | check | flash` — firmware (`can-usb`, `vcm`, `bms`).
- `just <ecu> erc | drc | export` — KiCad checks and schematic export.
- `just bootloader ci` — build the bootloader for every MCU family.
- `just wiring check | render` and `just site build | serve`.
- `just docs` — generate all artifacts and build the site.

Add new tasks as recipes, not as steps in a workflow file. Workflows only
install tools, restore caches and call `just`.

### Which checks a change needs

| Changed | Run |
| --- | --- |
| `crates/` | `just host ci`, then `just <ecu> build` for every firmware that uses the crate and `just bootloader ci` if the bootloader does |
| `apps/`, `tools/`, `ecus/can-usb/host/` | `just host ci` |
| `ecus/<ecu>/firmware/` | `just <ecu> ci-firmware` |
| `ecus/<ecu>/hardware/` | `just <ecu> ci-hardware` |
| `ecus/bootloader/` | `just bootloader ci` |
| `wiring/` | `just wiring check` |
| `site/` | `just site build`, or `just docs` when the page embeds generated artifacts |
| a `justfile`, `.github/` | the recipes you changed, and `actionlint` for workflows |

## Rules

- **Vertical changes are atomic.** A change to a CAN message, parameter or
  protocol updates the shared crate, every firmware and host user, and the
  documentation in the same commit.
- **Documentation lives in `site/`.** Keep READMEs to what a directory is and
  how to build it. Do not write design documentation elsewhere. Use the
  `unslop` skill on documentation prose you write.
- **Generated files are not committed.** Schematic exports and wiring renders
  land in `site/static/generated/`, which is ignored and rebuilt by CI.
- **Never flash, power or otherwise drive hardware unless asked to.** `flash`
  recipes are for the user to run.
- **The VCM and BMS are safety-relevant.** Call out any change that touches
  contactor control, precharge, current or voltage limits, or fault handling,
  and cover it with a host test.
- **Logic is tested on the host.** Put behaviour in a `crates/` library with
  tests; keep firmware binaries to wiring peripherals and tasks together.
- When adding a component, add its `just` module to the root `justfile` and a
  workflow with path filters under `.github/workflows/`.

## Task tracking

Work is tracked with [Bears](https://github.com/LHelge/bea-rs): tasks are
Markdown files under `.bears/`, committed with the code they describe. Bears
is registered as an MCP server in `.mcp.json`; use its tools, or the `bea`
CLI verbs of the same names when the server is not connected.

- `list_ready` shows tasks whose dependencies are done. Pick from there.
- `start_task` before working on a task; `release_task` if you stop without
  finishing, so it returns to the pool.
- `review_task` when the work is done and needs the user's review;
  `complete_task` only when it needs none or the user has approved it.
- `create_task` for follow-up work you find but should not do now. Use
  `propose_task` when unsure it belongs in the backlog.
- Group a vertical feature as an epic with one child task per component, and
  record ordering with dependencies.
- Task IDs are hierarchical (`a1b` → `a1b.c2`). Moving a task or deleting an
  epic renames tasks; use the IDs from the returned `renamed` map.
- A high `attempts` count on a task means earlier tries failed. Take a
  different approach or ask.
- Reference the task with a `Task: <id>` trailer, and commit the task file's status
  change together with the work.
- Tag a task `manual` when only the user can do it: KiCad schematic or layout
  work, flashing, anything measured on a bench or in the car, ordering parts,
  releases. Agents never start a `manual` task.
- Plan with the `bears-planning` skill. `/implement-epic <id>` runs a planned
  epic to a pull request with one `task-implementer` and one `task-reviewer`
  subagent per task.

## Commits and releases

Conventional Commits: `type(scope): summary`. One logical change per commit.

- **Types:** `feat`, `fix`, `perf`, `docs`, `refactor`, `test`, `build`, `ci`,
  `chore`. Wiki pages and blog posts are both `docs(site)`.
- **Scopes**, from this list only: `site`, `wiring`, `crates`, `can-usb`,
  `vcm`, `bms`, `bootloader`, `manager`, `ci`, `repo`. KiCad work takes the
  unit's scope with `-hw`: `fix(bms-hw): …`. A commit that spans components
  takes the scope where the change originates, usually `crates`.
- **Breaking (`!`)** means a change to a CAN message, a parameter, the
  bootloader protocol or the flash layout. Add a `BREAKING CHANGE:` footer
  that says which units must be reflashed and in what order.
- **Task reference** as a trailer on its own line: `Task: a1b.c2`.
- Write the subject for someone reading release notes. Release notes are
  generated per component with `just changelog <component>`, which selects
  commits by the paths they touch, not by scope.

Branches are `<type>/<short-slug>`, with the epic ID appended when there is
one. Code reaches `main` through a pull request, merged by rebase: no merge
commits and no squashing, so the per-component commits survive. Site content
and small fixes may be committed to `main` directly.

Tags are per component and never moved:

- `<unit>-fw-vX.Y.Z` and `bootloader-fw-vX.Y.Z` for firmware, `manager-vX.Y.Z`
  for the app. Major means wire-incompatible with the previous version.
- `<unit>-hw-revX` on the exact commit whose files were sent to fabrication.

Agents never create tags or releases.

Images committed to `site/` are resized web formats (JPEG or WebP, at most
2000 px on the long side). No originals, no Git LFS.
