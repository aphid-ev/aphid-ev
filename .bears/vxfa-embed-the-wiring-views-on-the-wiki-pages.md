---
id: vxfa
title: Embed the wiring views on the wiki pages
status: review
priority: P2
created: "2026-11-21T23:10:26.986915749Z"
updated: "2026-11-21T23:15:21.040540033Z"
tags:
  - site
  - wiring
depends_on:
  - sv2a
attempts: 1
---

aphid 0.6.0 inlines SVG images marked `#inline`, so the unstyled wirebug renders in site/static/generated/wiring can be put into wiki pages and styled by the theme.

Done when:
- aphid is pinned at v0.6.0 and the aphid skills are regenerated;
- the theme styles `.wirebug` drawings in the site's palette;
- every wiring view is embedded on the wiki page that covers it;
- `just docs` passes.