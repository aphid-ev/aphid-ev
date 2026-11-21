# Aphid EV

Everything that goes into the electric conversion of a classic VW Beetle:
documentation, wiring, ECU hardware and firmware, and the desktop tooling to
manage it.

The documentation site at <https://bladlus.se> is built from [`site/`](site/)
and is the single source of truth for the project. READMEs in this repository
only say what a directory is and how to build it.

## Layout

| Path | What |
| --- | --- |
| [`site/`](site/) | Blog and wiki for bladlus.se, built with [aphid](https://aphid.lhelge.se) |
| [`wiring/`](wiring/) | Vehicle schematics and harness drawings, built with [wirebug](https://github.com/LHelge/wirebug) |
| [`crates/`](crates/) | Shared `no_std` Rust crates used by firmware and host code |
| [`ecus/`](ecus/) | One directory per ECU: KiCad hardware and Rust firmware side by side |
| [`apps/`](apps/) | Desktop application for managing the ECUs over CAN |
| [`tools/`](tools/) | Small host-side CLIs used by the build |

aphid and wirebug are developed in their own repositories; this repository
only uses them.

## Working in the repository

All tasks go through [`just`](https://just.systems). CI runs the same recipes.

```sh
just                  # list every recipe, including per-component modules
just host ci          # format, lint and test the host workspace
just vcm build        # build one ECU's firmware
just vcm export       # export one ECU's schematics for the site
just site serve       # live-reload the site
just docs             # generate all artifacts and build the site
just ci               # everything CI runs
```

Required tools: a Rust toolchain via rustup, `just`, `aphid`, `wirebug`,
KiCad 10 (`kicad-cli`) and `probe-rs` for flashing. `just install-aphid` and
`just install-wirebug` install the versions CI uses.

## Rust workspaces

The root `Cargo.toml` is the host workspace: everything that runs on a desktop,
plus the shared crates so their tests run on the host. Each firmware is its own
small workspace with its own target, linker script, profiles and lock file, and
depends on the shared crates by path.

## Releases

Components are versioned and tagged separately: `<component>-fw-vX.Y.Z` for
firmware, `<component>-hw-revX` for board revisions and `manager-vX.Y.Z` for
the desktop app.
