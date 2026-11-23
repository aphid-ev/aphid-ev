---
title: Cab loom
category: Wiring
tags: [wiring, vcm]
created: 2026-11-23
---

The cab loom carries the driver's inputs to the [[vcm|VCM]]. Each input
hangs off the loom on its own branch behind a Deutsch DT inline connector,
so the branch unplugs as one. The cab loom carries the female half of each
inline connector and the branch carries the male half.

The [[vcm#io|VCM I/O]] section has the schematic of these circuits, and
[[wiring]] explains how to read the drawings.

The 12 V feeds from the fuse boxes to the brake and gear branches are not
drawn, because the fuse boxes have no connectors in the model yet.

# Key branch

The key loom runs about 2 m to the dash and carries the key's run and
start signals. The key takes its feed from the Beetle's own wiring. See
[[vcm#ignition]].

![Harness drawing of the cab-loom side of the key branch: the VCM key connector to the inline connector](/static/generated/wiring/cab_loom_key_branch.svg#inline "Cab loom, key branch")

![Harness drawing of the key loom from the inline connector to the key switch](/static/generated/wiring/key_loom.svg#inline "Key loom")

# Pedal branch

A short stub connects the [[accelerator|accelerator sensor]].

![Harness drawing of the cab-loom side of the pedal branch: the VCM accelerator connector to the inline connector](/static/generated/wiring/cab_loom_pedal_branch.svg#inline "Cab loom, pedal branch")

![Harness drawing of the pedal stub from the inline connector to the accelerator pedal](/static/generated/wiring/pedal_stub.svg#inline "Pedal stub")

# Brake branch

The brake loom runs about 2 m to the two brake switches. Each switch
terminal takes a single female spade. See [[LV system#brake-switch-feeds]]
for the two feeds.

![Harness drawing of the cab-loom side of the brake branch: the VCM brake connector to the inline connector](/static/generated/wiring/cab_loom_brake_branch.svg#inline "Cab loom, brake branch")

![Harness drawing of the brake loom from the inline connector to the two brake sensors](/static/generated/wiring/brake_loom.svg#inline "Brake loom")

# Gear branch

The gear loom runs about 2 m to the [[direction|gear lever]] and the
parking brake switch on the tunnel. The feed splits at a splice in the gear
loom.

![Harness drawing of the cab-loom side of the gear branch: the VCM gear connector to the inline connector](/static/generated/wiring/cab_loom_gear_branch.svg#inline "Cab loom, gear branch")

![Harness drawing of the gear loom from the inline connector to the gear lever and the parking brake switch](/static/generated/wiring/gear_loom.svg#inline "Gear loom")
