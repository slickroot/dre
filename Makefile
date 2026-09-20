.PHONY: build test fmt clippy install

build:
	nix develop --command cargo build

test:
	nix develop --command cargo test

fmt:
	nix develop --command cargo fmt

clippy:
	nix develop --command cargo clippy

install:
	nix develop --command cargo install --path . --debug --force --root $(HOME)/.local
