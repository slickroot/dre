# 087: Website Builders Download The Wasm Build From The Latest Release

As a website builder, when a new dre version is released, I can download `dre-web.zip` from the GitHub release page, so I can host dre in my own website.

## Acceptance Criteria

- Pushing to the `release` branch publishes a GitHub release that includes `dre-web.zip` next to the `dre` binaries.
- The zip contains the four wasm files (the `.wasm`, the `.js` glue, and the two type files) from an optimized build.
- The zip is named `dre-web.zip` with no version, so `.../releases/latest/download/dre-web.zip` always gives the newest build.
- If the wasm build fails, the whole release stops and nothing is published.

## Technical Design
