ARCH     ?= riscv64
PLATFORM ?= xemu
MODE     ?= release

export DEBUG ?= n
export LOG   ?= off

K    = alu-test
SRCS = tests/test.c

# Auto-generate test.c from gen_alu_test.c if missing.
# Must run before $(AM_HOME)/Makefile is included, so use $(shell).
# Requires two make invocations on a truly clean checkout.
ifeq ($(wildcard tests/test.c),)
$(info Generating tests/test.c ...)
$(shell mkdir -p tests && cc gen_alu_test.c -o /tmp/gen_alu_test && /tmp/gen_alu_test > tests/test.c && rm -f /tmp/gen_alu_test)
# Re-exec so VPATH picks up the newly created file.
.DEFAULT:
	@$(MAKE) $@
.PHONY: .DEFAULT
else
include $(AM_HOME)/Makefile
endif

clean::
	@rm -rf tests/
