# KiCad recipes shared by the ECUs. Imported by an ECU's justfile, which must
# set `ecu` to its directory name. Recipes run relative to that directory and
# cover every KiCad project found under ./hardware.

generated := source_directory() / '../site/static/generated/hardware'

# Run the electrical rules check on every schematic
erc:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob globstar
    for pro in hardware/**/*.kicad_pro; do
        sch="${pro%.kicad_pro}.kicad_sch"
        report="$(mktemp)"
        kicad-cli sch erc --severity-error --exit-code-violations -o "$report" "$sch" || { cat "$report"; exit 1; }
    done

# Run the design rules check on every board
drc:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob globstar
    for pro in hardware/**/*.kicad_pro; do
        pcb="${pro%.kicad_pro}.kicad_pcb"
        [ -f "$pcb" ] || continue
        report="$(mktemp)"
        kicad-cli pcb drc --schematic-parity --severity-error --exit-code-violations -o "$report" "$pcb" || { cat "$report"; exit 1; }
    done

# Export schematics (PDF and SVG) for the site
export:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob globstar
    out="{{ generated }}/{{ ecu }}"
    rm -rf "$out"
    for pro in hardware/**/*.kicad_pro; do
        name="$(basename "$pro" .kicad_pro)"
        sch="${pro%.kicad_pro}.kicad_sch"
        mkdir -p "$out/$name"
        kicad-cli sch export pdf -o "$out/$name/schematic.pdf" "$sch"
        kicad-cli sch export svg --no-background-color --exclude-drawing-sheet -o "$out/$name" "$sch"
    done

# What CI runs for the hardware
ci-hardware: erc drc
