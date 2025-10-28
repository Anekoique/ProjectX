config_args := \
  $(AM_HOME)/xconfig/configs/defconfig.toml $(PLAT_CONFIG) \
  -w 'arch="$(ARCH)"' \
  -w 'platform="$(PLATFORM)"' \
  -o "$(OUT_CONFIG)"

define defconfig
  $(call run_cmd,axconfig-gen,$(config_args))
endef

ifeq ($(wildcard $(OUT_CONFIG)),)
  define oldconfig
    $(call defconfig)
  endef
else
  define oldconfig
    $(if $(filter "$(PLATFORM)",$(shell axconfig-gen "$(OUT_CONFIG)" -r platform)),\
         $(call run_cmd,axconfig-gen,$(config_args) -c "$(OUT_CONFIG)"),\
         $(call defconfig))
  endef
endif

defconfig: _axconfig-gen
	$(call defconfig)

oldconfig: _axconfig-gen
	$(call oldconfig)

_axconfig-gen:
ifeq ($(shell axconfig-gen --version 2>/dev/null),)
	$(call run_cmd,RUSTFLAGS="" cargo,install axconfig-gen)
endif

.PHONY: _axconfig-gen
