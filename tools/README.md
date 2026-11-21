# Tools

Host-side CLIs used by the build, as members of the host workspace. Anything
with real logic (packing a firmware image, generating documentation tables
from the shared crates) goes here as Rust, not as shell in a justfile.
