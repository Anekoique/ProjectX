PROJECT := xemu
LOG ?= trace
ARCH ?= riscv64
BATCH ?= n
FILE ?=

export X_LOG=$(LOG)
export X_ARCH=$(ARCH)
export X_MODE=$(BATCH)
export X_FILE=$(FILE)

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
