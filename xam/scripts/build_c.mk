XLIB_HOME      ?= $(abspath $(AM_HOME)/../xlib)
LIBXLIB        := $(XLIB_HOME)/build/$(ARCH)-$(PLATFORM)-$(MODE)/libxlib.a

OBJS            = $(addprefix $(OUT_DIR)/, $(addsuffix .o, $(KERNEL_NAME)))
LINKAGE        += $(OBJS) $(LIBXLIB) $(LIBXHAL)

INC_PATH       += $(XLIB_HOME)/include $(WORK_DIR)/include $(KERNEL_DIR)/../include
INCFLAGS       += $(addprefix -I, $(INC_PATH))

CFLAGS         += -Wall -Werror 
CFLAGS         += -ffreestanding -fno-builtin -fno-stack-protector
CFLAGS         += $(INCFLAGS)
CXXFLAGS       += ${CFLAGS}
ASFLAGS        += $(INCFLAGS)

ifeq ($(MODE), release)
  CFLAGS       += -O3
endif

ifeq ($(ARCH), riscv64)
	CFLAGS       += -march=rv64gc -mabi=lp64d -mcmodel=medany
else ifeq ($(ARCH), loongarch64)
	CFLAGS       +=
endif

$(OUT_DIR)/%.o: %.c
	@mkdir -p $(dir $@)
	@$(CC) $(CFLAGS) -c -o $@ $(realpath $<)

$(OUT_DIR)/%.o: %.cpp
	@mkdir -p $(dir $@)
	@$(CXX) $(CXXFLAGS) -c -o $@ $(realpath $<)

$(OUT_DIR)/%.o: %.cc
	@mkdir -p $(dir $@)
	@$(CXX) $(CXXFLAGS) -c -o $@ $(realpath $<)

$(OUT_DIR)/%.o: %.S
	@mkdir -p $(dir $@)
	@$(AS) $(ASFLAGS) -c -o $@ $(realpath $<)

$(LIBXLIB):
	@$(MAKE) -C $(XLIB_HOME) ARCH=$(ARCH) PLATFORM=$(PLATFORM) MODE=$(MODE)