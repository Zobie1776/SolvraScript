.PHONY: desktop arm64 test all

DESKTOP=./cargo-desktop.sh
ARM64=./cargo-arm64.sh

all: desktop arm64 test

desktop:
	$(DESKTOP)

arm64:
	$(ARM64)

test:
	cd .. && cargo test --all-targets
