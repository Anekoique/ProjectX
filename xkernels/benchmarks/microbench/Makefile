ARCH     ?= riscv64
PLATFORM ?= xemu
MODE     ?= release

export DEBUG ?= n
export LOG   ?= off

MAINARGS ?= ref

K    = microbench
SRCS = $(shell find src/ \( -name '*.c' -o -name '*.cc' -o -name '*.S' \))

CFLAGS  += -DMAINARGS=\"$(MAINARGS)\"
CFLAGS  += -Wno-maybe-uninitialized

include $(AM_HOME)/Makefile
