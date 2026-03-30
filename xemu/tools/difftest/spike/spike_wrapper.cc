/**
 * Spike difftest C wrapper — thin C-ABI layer over Spike's C++ internals.
 * Follows the REMU/ysyx pattern: simif + processor, no full sim_t.
 */

#include "spike_ffi.h"

#include <riscv/cfg.h>
#include <riscv/devices.h>
#include <riscv/encoding.h>
#include <riscv/processor.h>
#include <riscv/simif.h>
#include <riscv/trap.h>

#include <cstdlib>
#include <cstring>
#include <map>
#include <sstream>
#include <vector>

// Minimal simif — just memory, no MMIO
class difftest_simif_t : public simif_t {
  public:
    difftest_simif_t(std::vector<std::pair<reg_t, mem_t *>> mems, cfg_t *cfg)
        : mems_(std::move(mems)), cfg_(cfg) {}

    void set_proc(processor_t *proc) {
        harts_[0] = proc;
    }

    char *addr_to_mem(reg_t paddr) override {
        for (auto &[base, mem] : mems_) {
            if (paddr >= base && paddr < base + mem->size())
                return mem->contents(paddr - base);
        }
        return nullptr;
    }

    bool mmio_load(reg_t, size_t, uint8_t *) override {
        return false;
    }
    bool mmio_store(reg_t, size_t, const uint8_t *) override {
        return false;
    }
    void proc_reset(unsigned) override {}
    const cfg_t &get_cfg() const override {
        return *cfg_;
    }
    const std::map<size_t, processor_t *> &get_harts() const override {
        return harts_;
    }
    const char *get_symbol(uint64_t) override {
        return "";
    }

    std::vector<std::pair<reg_t, mem_t *>> &mems() {
        return mems_;
    }

  private:
    std::vector<std::pair<reg_t, mem_t *>> mems_;
    cfg_t *cfg_;
    std::map<size_t, processor_t *> harts_;
};

struct spike_ctx {
    std::string isa_str;
    cfg_t cfg;
    difftest_simif_t *simif;
    processor_t *proc;
};

static mem_t *find_mem(spike_ctx_t *ctx, uintptr_t addr, reg_t *out_base) {
    for (auto &[base, mem] : ctx->simif->mems()) {
        if (addr >= base && addr < base + mem->size()) {
            *out_base = base;
            return mem;
        }
    }
    return nullptr;
}

extern "C" {

spike_ctx_t *spike_init(const spike_mem_t *regions, size_t n, uint64_t init_pc, uint32_t /* xlen */,
                        const char *isa) {
    if (!regions || n == 0 || !isa)
        return nullptr;

    auto *ctx = new spike_ctx_t();
    ctx->isa_str = isa;
    ctx->cfg.isa = ctx->isa_str.c_str();
    ctx->cfg.priv = "msu";
    ctx->cfg.hartids = {0};
    ctx->cfg.mem_layout.clear();
    ctx->cfg.pmpregions = 16;
    ctx->cfg.pmpgranularity = reg_t(1) << PMP_SHIFT;

    std::vector<std::pair<reg_t, mem_t *>> mems;
    for (size_t i = 0; i < n; i++) {
        ctx->cfg.mem_layout.emplace_back(regions[i].base, regions[i].size);
        mems.push_back({(reg_t)regions[i].base, new mem_t(regions[i].size)});
    }

    ctx->simif = new difftest_simif_t(std::move(mems), &ctx->cfg);
    static std::ostringstream null_out;
    ctx->proc = new processor_t(ctx->cfg.isa, ctx->cfg.priv, &ctx->cfg, ctx->simif, 0, false,
                                nullptr, null_out);
    ctx->simif->set_proc(ctx->proc);

    // Set initial PC
    state_t *s = ctx->proc->get_state();
    s->pc = (reg_t)init_pc;
    for (int i = 0; i < 32; i++)
        s->XPR.write(i, 0);

    return ctx;
}

void spike_fini(spike_ctx_t *ctx) {
    if (!ctx)
        return;
    delete ctx->proc;
    for (auto &[_, mem] : ctx->simif->mems())
        delete mem;
    delete ctx->simif;
    delete ctx;
}

int spike_step(spike_ctx_t *ctx) {
    if (!ctx || !ctx->proc)
        return -1;
    try {
        ctx->proc->step(1);
        return 0;
    } catch (trap_t &) {
        // Spike handles traps internally; this catches only fatal ones
        return -1;
    }
}

void spike_get_pc(spike_ctx_t *ctx, uint64_t *out) {
    if (ctx && out)
        *out = (uint64_t)ctx->proc->get_state()->pc;
}

void spike_get_gpr(spike_ctx_t *ctx, uint64_t out[32]) {
    if (!ctx || !out)
        return;
    state_t *s = ctx->proc->get_state();
    for (int i = 0; i < 32; i++)
        out[i] = (uint64_t)s->XPR[i];
}

uint64_t spike_get_csr(spike_ctx_t *ctx, uint16_t addr) {
    if (!ctx)
        return 0;
    try {
        return (uint64_t)ctx->proc->get_csr((int)addr);
    } catch (...) {
        return 0;
    }
}

uint64_t spike_get_priv(spike_ctx_t *ctx) {
    if (!ctx)
        return 0;
    return (uint64_t)ctx->proc->get_state()->prv;
}

void spike_set_pc(spike_ctx_t *ctx, uint64_t pc) {
    if (ctx)
        ctx->proc->get_state()->pc = (reg_t)pc;
}

void spike_set_gpr(spike_ctx_t *ctx, const uint64_t gpr[32]) {
    if (!ctx || !gpr)
        return;
    state_t *s = ctx->proc->get_state();
    for (int i = 0; i < 32; i++)
        s->XPR.write(i, (reg_t)gpr[i]);
}

void spike_set_csr(spike_ctx_t *ctx, uint16_t addr, uint64_t val) {
    if (!ctx)
        return;
    try {
        ctx->proc->put_csr((int)addr, (reg_t)val);
    } catch (...) {}
}

void spike_copy_mem(spike_ctx_t *ctx, uintptr_t addr, const void *data, size_t len) {
    if (!ctx || !data)
        return;
    reg_t base;
    mem_t *mem = find_mem(ctx, addr, &base);
    if (!mem)
        return;
    reg_t offset = addr - base;
    if (offset + len > mem->size())
        return;
    mem->store(offset, len, const_cast<uint8_t *>(static_cast<const uint8_t *>(data)));
}

int spike_write_mem(spike_ctx_t *ctx, uintptr_t addr, const void *data, size_t len) {
    if (!ctx || !data)
        return -1;
    reg_t base;
    mem_t *mem = find_mem(ctx, addr, &base);
    if (!mem)
        return -1;
    reg_t offset = addr - base;
    if (offset + len > mem->size())
        return -1;
    mem->store(offset, len, const_cast<uint8_t *>(static_cast<const uint8_t *>(data)));
    return 0;
}

} // extern "C"
