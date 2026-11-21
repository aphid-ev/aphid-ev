---
id: sv2a
title: Move the wirebug model into wiring/ and render it on the site
status: done
priority: P2
created: "2026-11-21T12:51:14.851053117Z"
updated: "2026-11-21T23:24:46.610726180Z"
tags:
  - wiring
  - site
  - ci
attempts: 1
---

The vehicle wiring model (`aphid-evpack`) is still in its own repository, cloned at ~/dev/wiring. `wiring/` in the monorepo has only a justfile and README, so `just wiring check` fails with "no `wirebug.toml`".

Site.yml had a "Check and render wiring" step (`just wiring::check wiring::render`). It was removed in the site migration (PR #1) so the site could build and deploy.

Done when:
- the model lives in `wiring/` and `just wiring check` passes;
- `.github/workflows/site.yml` runs `just wiring::check wiring::render` again before `just site build`, and its header comment no longer says wiring is skipped;
- the old wiring repository and its GitHub Pages render are retired or point here.