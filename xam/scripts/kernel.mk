KERNEL        := $(K)
KERNEL_NAME   := $(basename $(KERNEL))
KERNEL_DIR    := $(abspath $(KERNEL))

WORK_DIR      := $(shell pwd)
BUILD_DIR     := $(WORK_DIR)/build
OUT_DIR       := $(BUILD_DIR)/$(ARCH)-$(PLATFORM)
OUT_ELF       := $(OUT_DIR)/$(KERNEL_NAME)_$(PLATFORM).elf
OUT_BIN       := $(patsubst %.elf,%.bin,$(OUT_ELF))
OUT_MAP       := $(patsubst %.elf,%.map,$(OUT_ELF))
OUT_TXT       := $(patsubst %.elf,%.txt,$(OUT_ELF))

CROSS_COMPILE ?= $(ARCH)-linux-musl-
AS            := $(CROSS_COMPILE)gcc
CC            := $(CROSS_COMPILE)gcc
CXX           := $(CROSS_COMPILE)g++
LD            := $(CROSS_COMPILE)ld
AR            := $(CROSS_COMPILE)ar
OBJDUMP       := $(CROSS_COMPILE)objdump
OBJCOPY       := $(CROSS_COMPILE)objcopy
RANLIB        := $(CROSS_COMPILE)ranlib
READELF       := $(CROSS_COMPILE)readelf

LD_SCRIPT     := $(ARTIFACT_DIR)/linker_$(PLATFORM).lds
LDFLAGS        = -T $(LD_SCRIPT) -Map $(OUT_MAP)
LINKAGE       := 

ifneq ($(wildcard $(KERNEL_DIR)/Cargo.toml),)
  include $(AM_HOME)/scripts/build.mk
else
  include $(AM_HOME)/scripts/build_c.mk
endif

$(OUT_ELF): $(LINKAGE) $(LD_SCRIPT)
	@mkdir -p $(dir $@)
	@$(LD) $(LDFLAGS) --start-group $(LINKAGE) --end-group -o $@

$(OUT_BIN): $(OUT_ELF)
	@$(OBJCOPY) --strip-all -O binary $< $@

disasm: $(OUT_ELF)
	@$(OBJDUMP) -d $< > $(OUT_TXT)

kernel: $(OUT_BIN) disasm

clean:: 
	@rm -rf $(BUILD_DIR)

.PHONY: kernel clean