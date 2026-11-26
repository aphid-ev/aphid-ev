# Firmware recipes shared by the ECUs. Imported by an ECU's justfile, so
# recipes run relative to that ECU's directory. The firmware is a standalone
# cargo workspace in ./firmware; its target and probe-rs runner come from
# firmware/.cargo/config.toml.

# Build release firmware
[working-directory('firmware')]
build:
    cargo build --release

# Check formatting and lints
[working-directory('firmware')]
check:
    cargo fmt --all --check
    cargo clippy --release -- -D warnings

# Flash and run on the connected board through probe-rs
[working-directory('firmware')]
flash:
    cargo run --release

# Remove the firmware's target directory. Skipped while there is no firmware.
[working-directory('firmware')]
clean:
    if [ -f Cargo.toml ]; then cargo clean; fi

# What CI runs for the firmware
ci-firmware: check build
