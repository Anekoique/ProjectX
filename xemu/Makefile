PROJECT := xemu
LOG     ?= trace
ARCH    ?= riscv64
FILE    ?=
FW      ?=
KERNEL  ?=
INITRD  ?=
FDT     ?=
DISK    ?=
HARTS   ?=
MODE    ?= release
DEBUG   ?= y
VERBOSE ?=

export X_LOG    = $(LOG)
export X_ARCH   = $(ARCH)
export X_MODE   = $(MODE)
export X_FILE   = $(FILE)
export X_FW     = $(FW)
export X_KERNEL = $(KERNEL)
export X_INITRD = $(INITRD)
export X_FDT    = $(FDT)
export X_DISK   = $(DISK)
export X_HARTS  = $(HARTS)

build_args-release := --release
build_args-perf    := --profile perf
verbose := $(if $(VERBOSE),--verbose,)
build_args := \
	-Z unstable-options \
	$(build_args-$(MODE)) \
	$(verbose)

ifeq ($(DEBUG),y)
  feature_args := --features debug
endif

ifeq ($(DIFFTEST),1)
  feature_args += --features difftest
endif

all: run

run:
	cargo run $(build_args) $(feature_args)

clippy:
	cargo clippy

fmt:
	cargo fmt --all

test:
	cargo test

clean:
	cargo clean

.PHONY: all run clippy fmt test clean
