PROJECT := xemu
LOG     ?= trace
ARCH    ?= riscv64
BATCH   ?= n
FILE    ?=
MODE    ?= release
VERBOSE ?=

export X_LOG   = $(LOG)
export X_ARCH  = $(ARCH)
export X_MODE  = $(MODE)
export X_FILE  = $(FILE)
export X_BATCH = $(BATCH)

build_args-release := --release
build_args := \
	-Z unstable-options \
	$(build_args-$(MODE)) \
	$(verbose)

all: run

run:
	cargo run $(build_args)

clippy:
	cargo clippy

fmt:
	cargo fmt --all

test:
	cargo test -p xcore

clean:
	cargo clean

.PHONY: all run clean
