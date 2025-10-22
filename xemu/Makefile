PROJECT := xemu
LOG ?= off
ARCH ?= riscv32
FEATURES ?= $(ARCH)

export X_LOG=$(LOG)
export X_ARCH=$(ARCH)

all: run

run:
	cargo run

clippy:
	cargo clippy

fmt:
	cargo fmt --all

test:
	cargo test -p xcore

clean:
	cargo clean

.PHONY: all run clean
