# linux.mk — Download pre-built Linux kernel, build minimal initramfs

LINUX_URL  := https://toolchains.bootlin.com/downloads/releases/toolchains/riscv64-lp64d/test-system
LINUX_VER  := riscv64-lp64d--glibc--bleeding-edge-2024.02-1
LINUX_DIR  := linux
LINUX_IMG  := $(LINUX_DIR)/Image
LINUX_RD   := $(LINUX_DIR)/initramfs.cpio.gz
INIT_SRC   := patches/linux/init.c
INIT_ELF   := $(LINUX_DIR)/init

.PHONY: fetch-linux build-linux run-linux clean-linux

fetch-linux:
	@mkdir -p $(LINUX_DIR)
	@if [ ! -f $(LINUX_IMG) ]; then \
		echo "Downloading Linux Image..."; \
		curl -L -o $(LINUX_IMG) $(LINUX_URL)/$(LINUX_VER)-Image; \
	fi

$(INIT_ELF): $(INIT_SRC)
	$(CROSS)gcc -nostdlib -static -ffreestanding -O2 \
		-march=rv64imac_zicsr -mabi=lp64 \
		-Wl,-Ttext=0x10000 -o $@ $<

$(LINUX_RD): $(INIT_ELF)
	@echo "Creating initramfs..."
	@rm -rf $(LINUX_DIR)/_root
	@mkdir -p $(LINUX_DIR)/_root/proc $(LINUX_DIR)/_root/dev $(LINUX_DIR)/_root/sys
	@cp $< $(LINUX_DIR)/_root/init && chmod +x $(LINUX_DIR)/_root/init
	@cd $(LINUX_DIR)/_root && find . | cpio -o -H newc 2>/dev/null | gzip > ../initramfs.cpio.gz
	@rm -rf $(LINUX_DIR)/_root

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
