# xvisor Makefile — Type-1 RISC-V hypervisor.
#
# Targets:
#   make            -> alias of `make build` (release)
#   make build      -> cargo build --release for riscv64gc-unknown-none-elf
#   make run        -> build + launch QEMU on -machine virt with H-extension
#   make trap-test  -> build with --features trap-canary, run under QEMU,
#                       grep stdout for the trap line + post-trap banner
#   make fmt        -> cargo fmt --all
#   make clippy     -> cargo clippy with -D warnings
#   make test       -> no-op (xvisor is bin-only; host test harness lands later)
#   make clean      -> cargo clean

SHELL          := /bin/bash
.SHELLFLAGS    := -e -o pipefail -c

CARGO          ?= cargo
TARGET         ?= riscv64gc-unknown-none-elf
MODE           ?= release
OUT_BIN        := target/$(TARGET)/$(MODE)/xvisor

QEMU_SYSTEM    ?= qemu-system-riscv64
QEMU_MACHINE   ?= virt
QEMU_CPU       ?= rv64,h=true
QEMU_HARTS     ?= 1
QEMU_MEM       ?= 256M
QEMU_BIOS      ?= default

QEMU_FLAGS = \
  -nographic \
  -machine $(QEMU_MACHINE) \
  -cpu $(QEMU_CPU) \
  -smp $(QEMU_HARTS) \
  -m $(QEMU_MEM) \
  -bios $(QEMU_BIOS) \
  -kernel $(OUT_BIN)

TRAP_LOG       := /tmp/xvisor-trap.log
TRAP_LINE_RE   := ^xvisor: trap cause=0x3 sepc=0x[0-9a-f]+ stval=0x[0-9a-f]+$$
BANNER_RE      := ^xvisor: hello from HS-mode \(hartid=0, dtb=0x[0-9a-f]+\)$$

build:
	$(CARGO) build --$(MODE) --target $(TARGET)

run: build
	$(QEMU_SYSTEM) $(QEMU_FLAGS)

trap-test:
	$(CARGO) build --$(MODE) --target $(TARGET) --features trap-canary
	$(QEMU_SYSTEM) $(QEMU_FLAGS) | tee $(TRAP_LOG)
	@grep -E '$(TRAP_LINE_RE)' $(TRAP_LOG)
	@grep -E '$(BANNER_RE)' $(TRAP_LOG)

fmt:
	$(CARGO) fmt --all

clippy:
	$(CARGO) clippy --target $(TARGET) --bins -- -D warnings

test:
	@echo "xvisor: no host-runnable tests yet"

clean:
	$(CARGO) clean

.DEFAULT_GOAL := build
.PHONY: build run trap-test fmt clippy test clean
