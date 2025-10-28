OBJS            = $(addprefix $(OUT_DIR)/, $(addsuffix .o, $(KERNEL_NAME)))
LINKAGE        += $(OBJS) $(LIBXHAL)

INC_PATH       += $(WORK_DIR)/include
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