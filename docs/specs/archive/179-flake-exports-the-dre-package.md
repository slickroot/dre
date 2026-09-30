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
- On the VPS, `ssh dre.elaich.com` on port 22 opens dre, served by a systemd unit that runs the flake package. Admin SSH keeps working on port 2222.
- `nix flake update dre` plus a rebuild restarts the service on the new build, and `nixos-rebuild switch --rollback` brings the old one back.

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

Server side, for reference (not part of this repo). The VPS flake takes dre as an input and passes `inputs` to its modules:

```nix
inputs.dre.url = "github:slickroot/dre";

outputs = { nixpkgs, ... }@inputs: {
  nixosConfigurations.vps = nixpkgs.lib.nixosSystem {
    specialArgs = { inherit inputs; };
    modules = [ ./configuration.nix ];
  };
};
```

The service refers to the package by interpolation, not by a fixed path like `/opt/dre/dre`. Updating the input changes the store path in the unit, so the rebuild restarts `dre` and rollback covers it:

```nix
{ pkgs, lib, inputs, ... }:
let
  dre = inputs.dre.packages.${pkgs.system}.default;
in
{
  systemd.services.dre = {
    description = "dre over SSH";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];
    serviceConfig = {
      ExecStart = "${lib.getExe dre} serve --listen 0.0.0.0:22 --host-key /var/lib/dre/host_key --data-dir /var/lib/dre/diagrams";
      DynamicUser = true;
      StateDirectory = "dre";
      AmbientCapabilities = [ "CAP_NET_BIND_SERVICE" ];
      Restart = "always";
    };
  };

  # dre takes port 22, so admin SSH moves.
  services.openssh.ports = [ 2222 ];
  networking.firewall.allowedTCPPorts = [ 22 2222 ];
}
```

## QA

The whole flow, from `nix build` to `ssh dre.elaich.com` on port 22, is run once end to end. Every step has to pass before the spec is archived.

### 1. The package builds (dev machine, on the branch)

1. `nix build .#default -L` succeeds, and the log shows the test suite running and passing.
2. `ls result/bin` lists `dre`.
3. `nix run . -- serve --listen 127.0.0.1:2223 --host-key ./tmp/qa/host_key --data-dir ./tmp/qa/diagrams` starts. `--listen` is required: dre's default port, 2222, is the port admin SSH moves to below.
4. In another terminal, `ssh -p 2223 localhost` opens the editor. Quit it and stop the server.
5. `nix develop` still opens the dev shell, and `make test` passes.

### 2. The package builds from GitHub (VPS, after merge)

1. `nix build github:slickroot/dre --refresh -L` succeeds on the VPS (x86_64-linux), tests included.
2. `nix run github:slickroot/dre -- serve --listen 127.0.0.1:2223 --host-key /tmp/dre-qa/host_key --data-dir /tmp/dre-qa/diagrams` starts. `ssh -p 2223 localhost` on the VPS opens the editor. Stop the server and `rm -rf /tmp/dre-qa`.

### 3. Admin SSH moves to 2222 (VPS, its own rebuild)

This step is done on its own, before dre takes port 22, so a mistake can't lock you out.

1. If the VPS provider has a firewall outside NixOS, open TCP 2222 there.
2. Add `services.openssh.ports = [ 2222 ];` and `networking.firewall.allowedTCPPorts = [ 22 2222 ];` and run `nixos-rebuild switch --flake .#vps`. **Keep the current SSH session open.**
3. From the laptop, in a new terminal, `ssh -p 2222 <admin>@dre.elaich.com` logs in. Only close the old session after this works.
4. `ssh -p 22 dre.elaich.com` is refused, because nothing listens on 22 yet.

### 4. dre takes port 22 (VPS)

1. Add the `dre` input, `specialArgs` and `systemd.services.dre` shown in the Technical Design, then run `nix flake lock` and `nixos-rebuild switch --flake .#vps`.
2. `systemctl status dre` shows `active (running)`. `systemctl show dre -p ExecStart` shows a `/nix/store/…-dre-<version>/bin/dre` path, and `journalctl -u dre` has no errors.
3. `ss -tlnp | grep ':22 '` shows `dre` listening, and `sshd` is on `:2222` only.
4. `/var/lib/dre/host_key` exists (via the `/var/lib/private/dre` symlink that `DynamicUser` sets up).

### 5. Visitors get dre (laptop)

1. If `known_hosts` still has the old OpenSSH key for `dre.elaich.com`, `ssh` warns about a changed host key. Remove it with `ssh-keygen -R dre.elaich.com`. That's expected once and never again.
2. `ssh dre.elaich.com` opens the editor. Draw a box, quit.
3. `ssh dre.elaich.com` again: the box is still there.
4. `ssh -o PubkeyAuthentication=no -o IdentitiesOnly=yes dre.elaich.com` shows the "dre needs an SSH key" message.
5. On the VPS, `ls /var/lib/dre/diagrams` shows one directory per key fingerprint.

### 6. Restarts and reboots keep state (VPS)

1. `systemctl restart dre`, then `ssh dre.elaich.com` from the laptop: no host key warning, and the box is still there.
2. `reboot`. After it comes back, dre is running on 22, admin SSH works on 2222, and the box is still there.

### 7. Update and rollback (VPS)

1. Merge any commit to dre's `main`.
2. `nix flake update dre` changes the `dre` rev in the server's `flake.lock`.
3. `nixos-rebuild switch --flake .#vps` restarts `dre`: `systemctl show dre -p ExecStart` has a new store path, and `-p ActiveEnterTimestamp` is the time of the rebuild.
4. `nixos-rebuild switch --rollback` puts the previous store path back in `ExecStart`, restarts `dre`, and `ssh dre.elaich.com` still works.
