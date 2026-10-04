.PHONY: build test fmt clippy bench install run wasm serve

build:
	nix develop --command cargo build

test:
	nix develop --command cargo test -p dre -p types

fmt:
	nix develop --command cargo fmt

clippy:
	nix develop --command cargo clippy --workspace --all-targets --all-features -- -D warnings

bench:
	nix develop --command cargo bench

install:
	nix develop --command cargo install --path . --debug --force --root $(HOME)/.local

run:
	nix develop --command cargo run -- $(ARGS)

PROFILE_DIR = $(if $(RELEASE),release,debug)

wasm:
	nix develop --command cargo build -p dre-web --target wasm32-unknown-unknown $(if $(RELEASE),--release)
	nix develop --command wasm-bindgen target/wasm32-unknown-unknown/$(PROFILE_DIR)/dre_web.wasm --target web --out-dir web/pkg --out-name dre_web

serve:
	mkdir -p tmp
	nix develop --command cargo run -- serve --host-key ./tmp/host_key
