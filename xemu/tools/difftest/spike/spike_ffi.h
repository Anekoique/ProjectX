#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct spike_ctx spike_ctx_t;
typedef struct {
    uintptr_t base;
    size_t size;
} spike_mem_t;

spike_ctx_t *spike_init(const spike_mem_t *regions, size_t n, uint64_t init_pc,
                        uint32_t xlen, const char *isa);
void spike_fini(spike_ctx_t *ctx);
int spike_step(spike_ctx_t *ctx);
void spike_get_pc(spike_ctx_t *ctx, uint64_t *out);
void spike_get_gpr(spike_ctx_t *ctx, uint64_t out[32]);
uint64_t spike_get_csr(spike_ctx_t *ctx, uint16_t addr);
uint64_t spike_get_priv(spike_ctx_t *ctx);
void spike_set_pc(spike_ctx_t *ctx, uint64_t pc);
void spike_set_gpr(spike_ctx_t *ctx, const uint64_t gpr[32]);
void spike_set_csr(spike_ctx_t *ctx, uint16_t addr, uint64_t val);
void spike_copy_mem(spike_ctx_t *ctx, uintptr_t addr, const void *data, size_t len);
int spike_write_mem(spike_ctx_t *ctx, uintptr_t addr, const void *data, size_t len);

#ifdef __cplusplus
}
#endif
