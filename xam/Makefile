K              ?=
ARCH           ?= riscv64
PLATFORM       ?= xemu
MODE           ?= release
LOG            ?= warn
VERBOSE        ?= 

ifeq ($(ARCH), riscv64)
    TARGET     := riscv64gc-unknown-none-elf
else ifeq ($(ARCH), loongarch64)
    TARGET     := loongarch64-unknown-none
else
    $(error "ARCH" must be one of "riscv" or "loongarch")
endif

ifeq ($(wildcard $(AM_HOME)/xhal/src/lib.rs),)
    $(error $$AM_HOME must be an AbstractMachine repo)
endif

BUILD_KERNEL   := $(if $(strip $(K)),1,)
TARGET_DIR     ?= $(AM_HOME)/target
ARTIFACT_DIR   ?= $(TARGET_DIR)/$(TARGET)/$(MODE)
OUT_CONFIG     ?= $(AM_HOME)/.axconfig.toml
PLAT_CONFIG    ?= $(AM_HOME)/xconfig/configs/platforms/$(PLATFORM).toml
LIBXHAL        := $(ARTIFACT_DIR)/libxhal.a

export X_ARCH=$(ARCH)
export X_PLATFORM=$(PLATFORM)
export X_MODE=$(MODE)
export X_CONFIG_PATH=$(OUT_CONFIG)
export X_TARGET=$(TARGET)

include $(AM_HOME)/scripts/utils.mk
include $(AM_HOME)/scripts/config.mk
include $(AM_HOME)/scripts/cargo.mk
ifneq ($(BUILD_KERNEL),)
include $(AM_HOME)/scripts/kernel.mk
endif

all: lib $(if $(BUILD_KERNEL),kernel)

lib: $(LIBXHAL)

$(LIBXHAL): oldconfig
	$(call cargo_build,$(AM_HOME))

clippy: oldconfig
ifeq ($(origin ARCH), command line)
	$(call cargo_clippy,--target $(TARGET))
else
	$(call cargo_clippy)
endif

fmt:
	cargo fmt --all

clean::
	rm -rf $(OUT_CONFIG)
	$(call cargo_clean,$(AM_HOME))

.DEFAULT_GOAL := all
.PHONY: all defconfig oldconfig build clippy fmt clean

