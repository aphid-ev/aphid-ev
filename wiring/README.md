# Wiring

[wirebug](https://github.com/LHelge/wirebug) model of the vehicle's electrical
system: logical circuits, harnesses and connector pinouts. Needs the wirebug
release pinned in the root justfile (`just install-wirebug`).

```sh
just wiring check     # validate with warnings as errors, check formatting
just wiring fmt       # format the .wb files
just wiring serve     # live-reload preview
just wiring render    # embeddable SVGs into site/static/generated/wiring
just wiring pdf       # A4 PDF into wiring/out
just wiring skill     # regenerate the wirebug-dsl agent skill
```
