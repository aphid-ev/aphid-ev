---
id: thmn.6f
title: Generate the boot protocol tables on the wiki from the crate
status: blocked
priority: P3
created: "2026-11-22T09:23:05.912169916Z"
updated: "2026-11-23T20:24:18.750157601Z"
tags:
  - site
  - crates
attempts: 1
---

The identifier and error code tables on the `bootloader` wiki page are copied by hand from `crates/boot-protocol`, against the site rule that facts defined in code are generated. Add a tool in `tools/` that writes them to `site/static/generated/` and include them from the page.

## Blocked on aphid
aphid v0.6.0, the pinned release, can only inline SVG files from `/static/` (`![alt](/static/x.svg#inline)`). It has no way to include a generated Markdown or HTML fragment, which a generated table needs. This was checked against aphid's `docs/content/wiki/markdown.md` and the content skill template at v0.6.0. Per the repository rules, the capability belongs in aphid rather than a workaround here, such as rendering tables as SVG.

Unblocks when aphid can include a generated fragment, for example a Markdown include from `/static/`. The tables to generate: frame identifiers, error codes, image status codes and the image header layout, from `boot-protocol`'s constants and enums.