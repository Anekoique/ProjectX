# resource/Makefile — Build external boot artifacts for xemu
#
# Targets:
#   make opensbi  — build and boot OpenSBI firmware
#   make xv6      — build and boot xv6 kernel (ramdisk)
#   make linux    — download and boot Linux (pre-built Image + initramfs)
#   make dtb      — compile xemu.dts → xemu.dtb
#   make clean    — remove all build artifacts

XEMU_HOME := $(abspath ../xemu)
CROSS     := riscv64-unknown-elf-
DTB       := xemu.dtb

include opensbi.mk
include xv6.mk
include linux.mk

.PHONY: all dtb clean

all: dtb build-opensbi build-xv6

# --- DTB ---

dtb: $(DTB)

$(DTB): xemu.dts
	dtc -I dts -O dtb -o $@ $<

# --- Aliases ---

opensbi: run-opensbi
xv6: run-xv6
linux: run-linux

# --- Clean ---

clean: clean-opensbi clean-xv6 clean-linux
	rm -f $(DTB)
