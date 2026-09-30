# Flake exports the dre package

This is a technical spec, not a user story: it has no user-facing behaviour.

## Problem

`flake.nix` only exports a dev shell. To run `dre serve` on a NixOS VPS, the server config has to install dre from the release binaries by hand, outside Nix. It cannot take dre as a flake input and pin it like any other package.

## Acceptance Criteria

- `nix build github:slickroot/dre` produces `result/bin/dre`.
- `nix run github:slickroot/dre -- serve` starts the SSH server.
- A NixOS config can add dre as a flake input and use `dre.packages.${system}.default`. Updating it is `nix flake update dre` followed by a rebuild.
- `nix build` runs the test suite and passes on x86_64-linux.
- The package version always matches `Cargo.toml`.
- `nix develop` and every Makefile target behave as before.

## Technical Design

Decisions:

- **Package only.** The flake exports `packages.default` and nothing else. The systemd unit, port, user, state directory and firewall stay in the server config, which runs `${lib.getExe dre} serve --listen … --host-key … --data-dir …`. A `nixosModules.default` would be its own story.
- **nixpkgs toolchain.** `pkgs.rustPlatform.buildRustPackage` builds with the rustc from the pinned nixpkgs-unstable, which is cached on cache.nixos.org. The rust-overlay toolchain stays in the dev shell only.
- **Lockfile pinning.** `cargoLock.lockFile = ./Cargo.lock;` needs no `cargoHash` to maintain. `Cargo.lock` has no git sources, so no `outputHashes` either.
- **Tests run on build.** `doCheck` keeps its default, so a failing test stops a broken binary from reaching the server. A test that fails only in the sandbox gets made hermetic. Checks are not switched off.
- **Cargo.toml is the source of truth.** `pname` and `version` come from `lib.importTOML ./Cargo.toml`.
- **`meta.mainProgram = "dre"`**, so `nix run` and `lib.getExe` resolve the binary.
- **`src = ./.`** Inside a flake this is limited to git-tracked files, so `target/`, `web/pkg/` and `tmp/` are left out. A doc-only change triggering a rebuild is accepted.
- **Only the root crate is built.** `cargo build` at the workspace root builds the `dre` package. `types` is built as a dependency. `web` is vendored but not compiled.
- **No CI job.** CI stays Nix-free. A broken package shows up as a failed `nixos-rebuild`, which keeps the previous generation running.

Change, in `flake.nix` only: add `packages.default` next to `devShells.default`.

```nix
packages.default =
  let cargoToml = pkgs.lib.importTOML ./Cargo.toml;
  in pkgs.rustPlatform.buildRustPackage {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;
    src = ./.;
    cargoLock.lockFile = ./Cargo.lock;
    meta.mainProgram = "dre";
  };
```

Server side, for reference (not part of this repo):

```nix
inputs.dre.url = "github:slickroot/dre";
# ...
ExecStart = "${lib.getExe dre.packages.${pkgs.system}.default} serve --listen 0.0.0.0:22 --host-key /var/lib/dre/host_key --data-dir /var/lib/dre/diagrams";
```
