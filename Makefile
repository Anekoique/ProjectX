PROJECT := XEmu
LOG ?= off
ARCH ?= riscv32
FEATURES ?= $(ARCH)

export X_LOG=$(LOG)
export X_ARCH=$(ARCH)

all: run

run:
	@cd $(PROJECT) && cargo run

clippy:
	@cd $(PROJECT) && cargo clippy

fmt:
	@cd $(PROJECT) && cargo fmt --all

test:
	@cd $(PROJECT) && cargo test -p xcore

clean:
	@cd $(PROJECT) && cargo clean

.PHONY: all run clean
