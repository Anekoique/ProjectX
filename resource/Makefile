# resource/Makefile — Build external boot artifacts for xemu
#
# Targets:
#   make opensbi  — boot OpenSBI firmware only
#   make xv6      — boot xv6 kernel (ramdisk, interactive shell)
#   make linux    — boot Linux (OpenSBI + kernel + initramfs)
#   make dtb      — compile xemu.dts → xemu.dtb
#   make clean    — remove all build artifacts

XEMU_HOME := $(abspath ../xemu)
CROSS     := riscv64-unknown-elf-
DTB       := xemu.dtb
DTB_2HART := xemu-2hart.dtb

include opensbi.mk
include xv6.mk
include linux.mk
include debian.mk

.PHONY: all dtb clean opensbi xv6 linux debian linux-2hart debian-2hart

all: dtb build-opensbi build-xv6

dtb: $(DTB) $(DTB_2HART)
$(DTB): xemu.dts
	dtc -I dts -O dtb -o $@ $<
$(DTB_2HART): xemu-2hart.dts
	dtc -I dts -O dtb -o $@ $<

opensbi: run-opensbi
xv6:     run-xv6
linux:   run-linux
linux-2hart:  run-linux-2hart
debian-2hart: run-debian-2hart

clean: clean-opensbi clean-xv6 clean-linux clean-debian
	rm -f $(DTB) $(DTB_2HART)
