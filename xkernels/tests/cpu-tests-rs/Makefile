.PHONY: all run clean latest $(ALL)

export BATCH ?= y
export LOG   ?= off

RESULT = .result
$(shell > $(RESULT))

COLOR_RED   = \033[1;31m
COLOR_GREEN = \033[1;32m
COLOR_NONE  = \033[0m

ALL = $(notdir $(patsubst %/Cargo.toml,%,$(wildcard */Cargo.toml)))

all: $(addprefix Makefile., $(ALL))
	@echo "test list [$(words $(ALL)) item(s)]:" $(ALL)

$(ALL): %: Makefile.%

Makefile.%: %/Cargo.toml latest
	@printf "K = $*\nSRCS = $*\ninclude $${AM_HOME}/Makefile\n" > $@
	@if make -s -f $@ run; then \
		printf "[%14s] $(COLOR_GREEN)PASS$(COLOR_NONE)\n" $* >> $(RESULT); \
	else \
		printf "[%14s] $(COLOR_RED)***FAIL***$(COLOR_NONE)\n" $* >> $(RESULT); \
	fi
	-@rm -f Makefile.$*

run: all
	@cat $(RESULT)
	@grep -q "FAIL" $(RESULT); FAIL=$$?; rm $(RESULT); test $$FAIL -ne 0

clean:
	rm -rf Makefile.* build/

latest:
