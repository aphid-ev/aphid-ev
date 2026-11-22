---
id: thmn.6f
title: Generate the boot protocol tables on the wiki from the crate
status: open
priority: P3
created: "2026-11-22T09:23:05.912169916Z"
updated: "2026-11-22T10:52:54.243486265Z"
tags:
  - site
  - crates
---

The identifier and error code tables on the `bootloader` wiki page are copied by hand from `crates/boot-protocol`, against the site rule that facts defined in code are generated. Add a tool in `tools/` that writes them to `site/static/generated/` and include them from the page.