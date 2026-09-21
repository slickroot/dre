.PHONY: build test fmt clippy install demo

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

demo:
	nix develop --command cargo build -p dre-web --target wasm32-unknown-unknown
	nix develop --command wasm-bindgen target/wasm32-unknown-unknown/debug/dre_web.wasm --target web --out-dir examples/landing-page-editor-demo/pkg --out-name dre_web
