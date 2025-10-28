.PHONY: setup build-lite build-full build-tablet run-lite run-full lint test package

setup:
	cargo fetch

build-lite:
	cargo build -p nova_compositor --no-default-features --features lite

build-full:
	cargo build -p nova_compositor --features full

build-tablet:
	cargo build -p nova_compositor --features tablet

run-lite:
	cargo run -p nova_compositor --no-default-features --features lite

run-full:
	cargo run -p nova_compositor --features full

lint:
	cargo fmt --all -- --check && \
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all

package:
	cargo build --release --all-targets
