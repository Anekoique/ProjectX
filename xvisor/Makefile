# xvisor Makefile — Type-1 RISC-V hypervisor.
#
# Targets:
#   make           -> alias of `make build` (release)
#   make build     -> cargo build --release for riscv64gc-unknown-none-elf
#   make run       -> build + launch QEMU on -machine virt with H-extension
#   make fmt       -> cargo fmt --all
#   make clippy    -> cargo clippy with -D warnings
#   make test      -> cargo test (host-runnable; const-asserts mostly no-op)
#   make clean     -> cargo clean

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

build:
	$(CARGO) build --$(MODE) --target $(TARGET)

run: build
	$(QEMU_SYSTEM) $(QEMU_FLAGS)

fmt:
	$(CARGO) fmt --all

clippy:
	$(CARGO) clippy --target $(TARGET) --bins -- -D warnings

test:
	@echo "xvisor: no host-runnable tests yet"

clean:
	$(CARGO) clean

.DEFAULT_GOAL := build
.PHONY: build run fmt clippy test clean
