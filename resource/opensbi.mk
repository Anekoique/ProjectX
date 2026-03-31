# opensbi.mk — Fetch and build OpenSBI fw_jump.bin

OPENSBI_REPO := https://github.com/riscv-software-src/opensbi.git
OPENSBI_TAG  := v1.3.1
OPENSBI_DIR  := opensbi
OPENSBI_FW   := $(OPENSBI_DIR)/build/platform/generic/firmware/fw_jump.bin

.PHONY: fetch-opensbi build-opensbi run-opensbi clean-opensbi

fetch-opensbi:
	@if [ ! -d $(OPENSBI_DIR) ]; then \
		echo "Cloning OpenSBI $(OPENSBI_TAG)..."; \
		git clone --depth 1 --branch $(OPENSBI_TAG) $(OPENSBI_REPO) $(OPENSBI_DIR); \
	else \
		echo "OpenSBI source already present."; \
	fi

build-opensbi: fetch-opensbi
	$(MAKE) -C $(OPENSBI_DIR) \
		CROSS_COMPILE=$(CROSS) \
		PLATFORM=generic \
		FW_JUMP_ADDR=0x80200000 \
		FW_JUMP_FDT_ADDR=0x87F00000 \
		CC="$(CROSS)gcc -std=gnu11"

run-opensbi: $(DTB) build-opensbi
	$(MAKE) -C $(XEMU_HOME) run \
		FW=$(abspath $(OPENSBI_FW)) \
		FDT=$(abspath $(DTB)) \
		DEBUG=n LOG=warn

clean-opensbi:
	rm -rf $(OPENSBI_DIR)/build
