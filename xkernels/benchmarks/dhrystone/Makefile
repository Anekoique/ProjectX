ARCH     ?= riscv64
PLATFORM ?= xemu
MODE     ?= release

export DEBUG ?= n
export LOG   ?= off

K    = dhrystone
SRCS = dry.c

# Dhrystone uses K&R-style function definitions
CFLAGS += -Wno-old-style-definition

include $(AM_HOME)/Makefile
