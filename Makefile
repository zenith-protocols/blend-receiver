CARGO_TARGET_DIR ?= target
export CARGO_TARGET_DIR
WASM := wasm/blend_receiver.wasm

default: build

build:
	stellar contract build --locked --package blend-receiver --out-dir wasm
	@ls -l $(WASM)
	@sha256sum $(WASM)

test: build
	cargo test --locked --workspace

fmt:
	cargo fmt --all

check: build
	cargo fmt --all -- --check
	cargo clippy --locked --workspace --all-targets -- -D warnings

clean:
	cargo clean
	rm -f $(WASM)

.PHONY: default build test fmt check clean
