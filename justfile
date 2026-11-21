# Aphid EV task runner.
#
# Recipes are the single entry point for local work and for CI: workflows only
# install tools, restore caches and call `just`. Each component is a module
# with its own justfile, so `just vcm build` works from the repository root
# and `just build` works from inside `ecus/vcm/`.

# Tool versions CI installs. Bump here to adopt a newer release.
aphid_version := 'v0.5.1'
wirebug_version := 'v0.1.0'
bin := env('HOME') / '.local/bin'

# Host workspace: shared crates, host library, desktop app, tools
mod host 'host.just'
# Documentation site (aphid)
mod site
# Vehicle wiring model (wirebug)
mod wiring
# CAN/USB interface
mod can-usb 'ecus/can-usb'
# Vehicle control module
mod vcm 'ecus/vcm'
# Battery management system
mod bms 'ecus/bms'
# ECU bootloader
mod bootloader 'ecus/bootloader'

[private]
default:
    @just --list --list-submodules

# Export KiCad schematics for every board into site/static/generated/hardware
hardware-export: can-usb::export vcm::export bms::export

# Generate every artifact the site embeds
artifacts: hardware-export wiring::render

# Generate all artifacts and build the site
docs: artifacts site::build

# Everything CI runs, across all components
ci: host::ci can-usb::ci vcm::ci bms::ci bootloader::ci wiring::check docs

# Release notes since a component's last tag (vcm, vcm-hw, bms, bms-hw, can-usb, can-usb-hw, bootloader, manager)
changelog component:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ component }}" in
        vcm | bms | can-usb)
            paths=("ecus/{{ component }}/firmware/**" "crates/**")
            tags='^{{ component }}-fw-v[0-9]' ;;
        vcm-hw | bms-hw | can-usb-hw)
            unit="{{ trim_end_match(component, '-hw') }}"
            paths=("ecus/$unit/hardware/**")
            tags="^$unit-hw-rev" ;;
        bootloader)
            paths=("ecus/bootloader/**" "crates/**")
            tags='^bootloader-fw-v[0-9]' ;;
        manager)
            paths=("apps/**" "ecus/can-usb/host/**" "crates/**")
            tags='^manager-v[0-9]' ;;
        *)
            echo "unknown component '{{ component }}'" >&2
            exit 1 ;;
    esac
    args=()
    for path in "${paths[@]}"; do args+=(--include-path "$path"); done
    git cliff --unreleased --tag-pattern "$tags" "${args[@]}"

# Install the pinned aphid release (Linux x86_64)
install-aphid:
    mkdir -p {{ bin }}
    curl -fsSL https://github.com/LHelge/aphid/releases/download/{{ aphid_version }}/aphid-{{ aphid_version }}-x86_64-unknown-linux-gnu.tar.gz | tar xz -C {{ bin }} aphid
    {{ bin }}/aphid --version

# Install the pinned wirebug release (Linux x86_64)
install-wirebug:
    mkdir -p {{ bin }}
    curl -fsSL -o {{ bin }}/wirebug https://github.com/LHelge/wirebug/releases/download/{{ wirebug_version }}/wirebug-linux-x86_64
    chmod +x {{ bin }}/wirebug
    {{ bin }}/wirebug --version
