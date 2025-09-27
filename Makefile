LOG ?= off
ARCH ?= riscv64
FEATURES ?= $(ARCH)

export X_LOG=$(LOG)
export X_ARCH=$(ARCH)

all: run

run:
	cargo run

clean:
	cargo clean

.PHONY: all run clean
