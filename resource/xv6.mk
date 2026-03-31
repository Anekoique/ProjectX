# xv6.mk — Fetch, patch, and build xv6-riscv with ramdisk

XV6_REPO    := https://github.com/mit-pdos/xv6-riscv.git
XV6_DIR     := xv6
XV6_KERNEL  := $(XV6_DIR)/kernel/kernel.bin
XV6_PATCH   := patches/xv6/ramdisk.patch
XV6_RAMDISK := patches/xv6/ramdisk.c
XV6_PATCHED := $(XV6_DIR)/.xemu_patched

.PHONY: fetch-xv6 build-xv6 run-xv6 clean-xv6

fetch-xv6:
	@if [ ! -d $(XV6_DIR) ]; then \
		echo "Cloning xv6-riscv..."; \
		git clone --depth 1 $(XV6_REPO) $(XV6_DIR); \
	else \
		echo "xv6 source already present."; \
	fi

$(XV6_PATCHED): $(XV6_PATCH) $(XV6_RAMDISK) | fetch-xv6
	cd $(XV6_DIR) && git checkout -- . 2>/dev/null || true
	cd $(XV6_DIR) && git apply ../$(XV6_PATCH)
	cp $(XV6_RAMDISK) $(XV6_DIR)/kernel/ramdisk.c
	@touch $@

build-xv6: $(XV6_PATCHED)
	$(MAKE) -C $(XV6_DIR) TOOLPREFIX=$(CROSS) kernel/kernel
	$(CROSS)objcopy -O binary $(XV6_DIR)/kernel/kernel $(XV6_KERNEL)

run-xv6: build-xv6
	$(MAKE) -C $(XEMU_HOME) run \
		FILE=$(abspath $(XV6_KERNEL)) \
		DEBUG=n LOG=warn

clean-xv6:
	@if [ -d $(XV6_DIR) ]; then \
		$(MAKE) -C $(XV6_DIR) clean 2>/dev/null || true; \
		rm -f $(XV6_KERNEL) $(XV6_PATCHED); \
	fi
