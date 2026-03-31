# linux.mk — Download pre-built Linux kernel + rootfs for xemu
#
# Downloads from bootlin.com: kernel Image + rootfs tarball.
# Converts rootfs to initramfs cpio, loads as initrd at 0x84000000.

LINUX_BASE_URL := https://toolchains.bootlin.com/downloads/releases/toolchains/riscv64-lp64d/test-system
LINUX_VERSION  := riscv64-lp64d--glibc--bleeding-edge-2024.02-1

LINUX_DIR      := linux
LINUX_IMAGE    := $(LINUX_DIR)/Image
LINUX_ROOTFS   := $(LINUX_DIR)/rootfs.tar
LINUX_INITRD   := $(LINUX_DIR)/initramfs.cpio.gz

.PHONY: fetch-linux build-linux run-linux clean-linux

fetch-linux:
	@mkdir -p $(LINUX_DIR)
	@if [ ! -f $(LINUX_IMAGE) ]; then \
		echo "Downloading Linux Image..."; \
		curl -L -o $(LINUX_IMAGE) $(LINUX_BASE_URL)/$(LINUX_VERSION)-Image; \
	fi
	@if [ ! -f $(LINUX_ROOTFS) ]; then \
		echo "Downloading rootfs..."; \
		curl -L -o $(LINUX_ROOTFS) $(LINUX_BASE_URL)/$(LINUX_VERSION)-rootfs.tar; \
	fi

$(LINUX_INITRD): $(LINUX_ROOTFS)
	@echo "Creating initramfs..."
	@rm -rf $(LINUX_DIR)/rootfs
	@mkdir -p $(LINUX_DIR)/rootfs
	@cd $(LINUX_DIR)/rootfs && tar xf ../rootfs.tar 2>/dev/null
	@printf '#!/bin/sh\nmount -t proc proc /proc\nmount -t sysfs sysfs /sys\nmount -t devtmpfs devtmpfs /dev\necho "Welcome to xemu Linux!"\nexec /bin/sh\n' > $(LINUX_DIR)/rootfs/init
	@chmod +x $(LINUX_DIR)/rootfs/init
	@cd $(LINUX_DIR)/rootfs && find . | cpio -o -H newc 2>/dev/null | gzip > ../initramfs.cpio.gz
	@rm -rf $(LINUX_DIR)/rootfs

build-linux: fetch-linux $(LINUX_INITRD)

run-linux: $(DTB) build-opensbi build-linux
	$(MAKE) -C $(XEMU_HOME) run \
		FW=$(abspath $(OPENSBI_FW)) \
		KERNEL=$(abspath $(LINUX_IMAGE)) \
		INITRD=$(abspath $(LINUX_INITRD)) \
		FDT=$(abspath $(DTB)) \
		DEBUG=n LOG=warn

clean-linux:
	rm -rf $(LINUX_DIR)
