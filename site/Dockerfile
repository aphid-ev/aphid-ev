# bladlus.se, served by aphid-server.
#
# Built and deployed to hoster by .github/workflows/site.yml after
# `just site build` has rendered the site into dist/. APHID_SERVER_VERSION is
# the aphid release pinned in the root justfile (`just aphid-server-version`),
# so the server matches the generator that built the site.
ARG APHID_SERVER_VERSION
FROM ghcr.io/lhelge/aphid-server:${APHID_SERVER_VERSION}
COPY dist /site
