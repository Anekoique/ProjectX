# xam/scripts/platforms/riscv64-qemu-virt.mk — runner for
# PLATFORM=riscv64-qemu-virt
#
# Boots the kernel under upstream QEMU on the `virt` machine.  All knobs are
# user-overridable from the command line: e.g. `make run HARTS=4 MEM=256M`.

QEMU_SYSTEM   ?= qemu-system-riscv64
QEMU_MACHINE  ?= virt
QEMU_MEM      ?= 128M
QEMU_CPU      ?= rv64
QEMU_HARTS    ?= 1
QEMU_BIOS     ?= none
QEMU_EXTRA    ?=

QEMU_FLAGS = \
  -nographic \
  -machine $(QEMU_MACHINE) \
  -cpu $(QEMU_CPU) \
  -smp $(QEMU_HARTS) \
  -m $(QEMU_MEM) \
  -bios $(QEMU_BIOS) \
  -kernel $(OUT_BIN) \
  $(QEMU_EXTRA)

run-platform: kernel
	@$(QEMU_SYSTEM) $(QEMU_FLAGS)

.PHONY: run-platform
