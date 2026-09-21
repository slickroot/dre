# 087: Website Builders Download The Wasm Build From The Latest Release

As a website builder, when a new dre version is released, I can download `dre-web.zip` from the GitHub release page, so I can host dre in my own website.

## Acceptance Criteria

- Pushing to the `release` branch publishes a GitHub release that includes `dre-web.zip` next to the `dre` binaries.
- The zip contains the four wasm files (the `.wasm`, the `.js` glue, and the two type files) from an optimized build.
- The zip is named `dre-web.zip` with no version, so `.../releases/latest/download/dre-web.zip` always gives the newest build.
- If the wasm build fails, the whole release stops and nothing is published.

## Technical Design

Only `.github/workflows/release.yml` changes. No Rust code and no Makefile change.

### New `wasm` job

- `needs: check`, runs in parallel with `build` on `ubuntu-latest`.
- Uses `dtolnay/rust-toolchain@stable` with `targets: wasm32-unknown-unknown`, and `Swatinem/rust-cache@v2`, like the `build` job. It does not use nix.
- Reads the `wasm-bindgen` crate version from the lockfile (`cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="wasm-bindgen") | .version'`) and runs `cargo install wasm-bindgen-cli --version <that> --locked`, so the CLI always matches the crate.
- Builds with `cargo build -p dre-web --target wasm32-unknown-unknown --release`, then `wasm-bindgen target/wasm32-unknown-unknown/release/dre_web.wasm --target web --out-dir web/pkg --out-name dre_web`. These are the same two commands as `make wasm RELEASE=1`, repeated in YAML. Optimized means a release build only: no `wasm-opt`.
- Zips from inside `web/pkg` with the four files named explicitly: `zip ../../dre-web.zip dre_web.js dre_web_bg.wasm dre_web.d.ts dre_web_bg.wasm.d.ts`. The layout is flat, with no version in the name. `zip` exits non-zero if a named file is missing, so the job fails.
- Uploads `dre-web.zip` with `actions/upload-artifact@v4` under the artifact name `dre-web`.

### `publish` job

- `needs: [check, build, wasm]`. A failed wasm job stops the release before `gh release create`, so nothing is published.
- The command doesn't change. `download-artifact` with `merge-multiple` puts `dre-web.zip` in `assets/`, and the existing `assets/*` glob attaches it next to the four `dre` binaries.
- The unversioned name makes `.../releases/latest/download/dre-web.zip` always resolve to the newest build.

### Collaborators

- `Makefile` `wasm` target: the source of the build commands, which are mirrored in the workflow.
- `Cargo.lock`: the source of the `wasm-bindgen` version.
- `check` job: unchanged. It still supplies the tag and refuses to overwrite an existing release.

### Verification

The workflow can't be unit-tested. Before merging, run the same commands locally: build with `RELEASE=1`, run the zip command, and check with `unzip -l` that the zip holds exactly the four files at its root. After the first release, download `releases/latest/download/dre-web.zip` and confirm it.
