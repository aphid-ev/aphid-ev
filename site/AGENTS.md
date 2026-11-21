# Site guidelines

aphid site for bladlus.se: blog posts are the build log, wiki pages are the
reference documentation. aphid's own content and theme conventions come from
`aphid agent`; this file only covers what is specific to the monorepo.

- This is where documentation goes. When code, hardware or wiring changes in a
  way a reader would care about, update the matching wiki page in the same
  commit.
- `static/generated/` is produced by `just artifacts` from the KiCad projects
  and the wirebug model. Never edit or commit files there; change the source
  and regenerate. Reference them by their `/static/generated/…` path.
- Facts that are defined in code (CAN messages, parameters, pinouts) should be
  generated or linked, not retyped. If a page needs a table that only exists
  in code, say so instead of copying values by hand.
- Vendor datasheets and CAD files are kept outside the repository. Copy the
  relevant facts into a wiki page and cite the source.
- Run the `unslop` skill over any prose you write or rewrite here before
  committing it. Apply it to your own text only; leave the user's wording and
  their blog posts as they are unless asked to edit them.
- `just site build` must pass: aphid fails the build on broken wiki-links.
