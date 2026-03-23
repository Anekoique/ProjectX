XLIB_HOME    := $(abspath .)
SRCS         := $(wildcard $(XLIB_HOME)/src/*.c)

ARCH         ?= riscv64
PLATFORM     ?= xemu
MODE         ?= release

CROSS_COMPILE ?= $(ARCH)-linux-musl-
CC            := $(CROSS_COMPILE)gcc
AR            := $(CROSS_COMPILE)ar

BUILD_DIR    := $(XLIB_HOME)/build/$(ARCH)-$(PLATFORM)-$(MODE)
OBJS         := $(patsubst $(XLIB_HOME)/src/%.c,$(BUILD_DIR)/%.o,$(SRCS))
LIBXLIB      := $(BUILD_DIR)/libxlib.a

CFLAGS       := -Wall -Werror -ffreestanding -fno-builtin -fno-stack-protector
CFLAGS       += -I$(XLIB_HOME)/include

ifeq ($(MODE), release)
  CFLAGS     += -O3
endif

ifeq ($(ARCH), riscv64)
  CFLAGS     += -march=rv64gc -mabi=lp64d -mcmodel=medany
endif

all: $(LIBXLIB)

$(LIBXLIB): $(OBJS)
	@mkdir -p $(dir $@)
	@$(AR) rcs $@ $^

$(BUILD_DIR)/%.o: $(XLIB_HOME)/src/%.c
	@mkdir -p $(dir $@)
	@$(CC) $(CFLAGS) -c -o $@ $<

clean:
	rm -rf $(XLIB_HOME)/build

.PHONY: all clean
