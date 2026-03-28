KERNEL_DIR       := $(abspath $(K))
KERNEL_NAME      := $(notdir $(KERNEL_DIR))
KERNEL_TARGET    := $(KERNEL_DIR)/target
KERNEL_ARTIFACT  := $(KERNEL_TARGET)/$(TARGET)/$(MODE)
LIBKERNEL        := $(KERNEL_ARTIFACT)/lib$(subst -,_,$(KERNEL_NAME)).a
LINKAGE          += $(LIBKERNEL)
LD_SCRIPT        := $(KERNEL_ARTIFACT)/linker_$(PLATFORM).lds

$(LIBKERNEL): oldconfig
	$(call cargo_build,$(KERNEL_DIR))

clean::
	$(call cargo_clean,$(KERNEL_DIR))
