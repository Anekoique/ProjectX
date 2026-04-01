# linux.mk — Download pre-built Linux kernel + buildroot rootfs, build initramfs

LINUX_URL  := https://toolchains.bootlin.com/downloads/releases/toolchains/riscv64-lp64d/test-system
LINUX_VER  := riscv64-lp64d--glibc--bleeding-edge-2024.02-1
LINUX_DIR  := linux
LINUX_IMG  := $(LINUX_DIR)/Image
LINUX_RD   := $(LINUX_DIR)/initramfs.cpio.gz
ROOTFS_DIR := $(LINUX_DIR)/rootfs
ROOTFS_TAR := $(LINUX_DIR)/rootfs.tar

.PHONY: fetch-linux build-linux run-linux clean-linux

fetch-linux:
	@mkdir -p $(LINUX_DIR)
	@if [ ! -f $(LINUX_IMG) ]; then \
		echo "Downloading Linux Image..."; \
		curl -L -o $(LINUX_IMG) $(LINUX_URL)/$(LINUX_VER)-Image; \
	fi
	@if [ ! -f $(ROOTFS_TAR) ]; then \
		echo "Downloading rootfs tarball..."; \
		curl -L -o $(ROOTFS_TAR) $(LINUX_URL)/$(LINUX_VER)-rootfs.tar; \
	fi

$(ROOTFS_DIR)/init: $(ROOTFS_TAR) patches/linux/init.sh
	@echo "Extracting rootfs..."
	@mkdir -p $(ROOTFS_DIR)
	@tar xf $(ROOTFS_TAR) -C $(ROOTFS_DIR)
	@cp patches/linux/init.sh $(ROOTFS_DIR)/init && chmod +x $(ROOTFS_DIR)/init

$(LINUX_RD): $(ROOTFS_DIR)/init
	@echo "Creating initramfs from buildroot rootfs..."
	@cd $(ROOTFS_DIR) && find . | cpio -o -H newc 2>/dev/null | gzip > $(abspath $@)

build-linux: fetch-linux $(LINUX_RD)

run-linux: $(DTB) build-opensbi build-linux
	$(MAKE) -C $(XEMU_HOME) run \
		FW=$(abspath $(OPENSBI_FW)) \
		KERNEL=$(abspath $(LINUX_IMG)) \
		INITRD=$(abspath $(LINUX_RD)) \
		FDT=$(abspath $(DTB)) \
		DEBUG=n LOG=warn

clean-linux:
	rm -rf $(LINUX_DIR)
