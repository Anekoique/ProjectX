# xam/scripts/platforms/xemu.mk — runner for PLATFORM=xemu
#
# Delegates to the xemu Rust workspace's top-level Makefile, passing the
# freshly built kernel binary via FILE=.  Requires $(XEMU_HOME) to point at
# the xemu repo root.

ifndef XEMU_HOME
  $(error XEMU_HOME is not set; cannot run PLATFORM=xemu)
endif

run-platform: kernel
	@$(MAKE) -C $(XEMU_HOME) run FILE=$(OUT_BIN)

.PHONY: run-platform
