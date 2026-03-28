.PHONY: all run clean latest $(ALL)

export BATCH ?= y
export LOG   ?= off

RESULT = .result
$(shell > $(RESULT))

COLOR_RED   = \033[1;31m
COLOR_GREEN = \033[1;32m
COLOR_NONE  = \033[0m

TIMEOUT ?= 60
TEST    ?=
SRCS     = $(shell find src/ -name "*.[cS]")
ALL      = u r t s p c e

name = $(patsubst u,uart-putc,$(patsubst r,timer-read,$(patsubst t,timer-irq,\
       $(patsubst s,soft-irq,$(patsubst p,plic-access,$(patsubst c,csr-warl,\
       $(patsubst e,trap-ecall,$(1))))))))

all: $(addprefix .run., $(if $(TEST),$(TEST),$(ALL)))

$(ALL): %: .run.%

.run.%: latest
	@echo 'K = amtest'                          >  .mk.$*
	@echo 'SRCS = $(SRCS)'                      >> .mk.$*
	@echo 'CFLAGS += -DMAINARGS="\"$*\""'       >> .mk.$*
	@echo 'include $$(AM_HOME)/Makefile'        >> .mk.$*
	@rm -rf build/
ifeq ($(BATCH),n)
	@make -f .mk.$* run
else
	@timeout $(TIMEOUT) make -s -f .mk.$* run 2>&1 > .output.$* || true
	@if grep -q "GOOD TRAP" .output.$*; then \
		printf "[%14s] $(COLOR_GREEN)PASS$(COLOR_NONE)\n" $(call name,$*) >> $(RESULT); \
	else \
		printf "[%14s] $(COLOR_RED)***FAIL***$(COLOR_NONE)\n" $(call name,$*) >> $(RESULT); \
	fi
	@rm -f .output.$*
endif
	-@rm -f .mk.$*

run: all
	@cat $(RESULT)
	@grep -q "FAIL" $(RESULT); F=$$?; rm -f $(RESULT); test $$F -ne 0

clean:
	rm -rf .mk.* .run.* build/ $(RESULT) .output*

latest:
