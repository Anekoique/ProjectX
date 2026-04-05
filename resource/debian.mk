# debian.mk — Boot Debian riscv64 via virtio-blk
#
# Downloads a pre-built Debian 13 (trixie) riscv64 ext4 image and boots it
# with the Bootlin kernel through virtio-blk.

DEBIAN_DIR  := debian
DEBIAN_DTB  := xemu-debian.dtb
DEBIAN_ARCH := $(DEBIAN_DIR)/debian-riscv64.img.tar.xz
DEBIAN_IMG  := $(DEBIAN_DIR)/debian-riscv64.img

DEBIAN_URL := https://github.com/Anekoique/Starry-Ros2/releases/download/debian-jazzy-minimal/debian-riscv64.img.tar.xz

.PHONY: fetch-debian build-debian run-debian clean-debian debian

$(DEBIAN_ARCH):
	@mkdir -p $(DEBIAN_DIR)
	@echo "Downloading Debian riscv64 image..."
	curl -L -o $@ "$(DEBIAN_URL)"

$(DEBIAN_IMG): $(DEBIAN_ARCH)
	@echo "Extracting image..."
	tar xf $< -C $(DEBIAN_DIR)
	@touch $@

$(DEBIAN_DTB): xemu-debian.dts
	dtc -I dts -O dtb -o $@ $<

fetch-debian: $(DEBIAN_IMG)

build-debian: $(DEBIAN_IMG) $(DEBIAN_DTB) build-opensbi fetch-linux

run-debian: build-debian
	$(MAKE) -C $(XEMU_HOME) run \
		FW=$(abspath $(OPENSBI_FW)) \
		KERNEL=$(abspath $(LINUX_IMG)) \
		FDT=$(abspath $(DEBIAN_DTB)) \
		DISK=$(abspath $(DEBIAN_IMG)) \
		DEBUG=n LOG=warn

debian: run-debian

clean-debian:
	rm -rf $(DEBIAN_DIR) $(DEBIAN_DTB)
