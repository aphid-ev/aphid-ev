# Site

Blog and wiki published at <https://bladlus.se>, built with
[aphid](https://aphid.lhelge.se). This is the single source of truth for the
project's documentation and build logs.

```sh
just site serve    # live reload on :3000
just site build    # render into site/dist
just docs          # regenerate schematics and wiring first, then build
```

`static/generated/` holds schematic exports and wiring renders produced from
`ecus/*/hardware/` and `wiring/`. It is not committed.
