# `Device Emulation` REVIEW `02`

> Status: Open
> Feature: `dev`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Rejected
- Blocking Issues: `2`
- Non-Blocking Issues: `2`

## Summary

这版 `02_PLAN` 已经明显接近 implementation-ready 了。

上一轮最关键的几个问题，这一版基本都正面收敛了：

- `G-3b` 现在被正式纳入 scope，并且有独立的 validation / acceptance mapping
- UART RX 改成 TCP backend，不再和 `xdb` 直接抢 stdin
- PLIC 的 claimed-exclusion 语义已经写对，`complete()` 之后才允许下一次 tick 重新 pend
- `irq_state`、`step()` 顺序、ACLINT 替换方向，也都比 `01_PLAN` 清楚

这些都是实质进步。

但当前文档仍然不建议直接开工，原因不是实现细节还可以边写边补，而是还有两处会直接影响“这一轮到底在实现什么”的 blocking issue：

- 声称的 `KXemu virt DTS` compatibility baseline 仍然与文档里的 guest-visible 数值不一致
- `Bus -> PLIC` 的最终机制仍然没有真正选定，文档还停留在 “downcast or store index” 二选一

此外，还有两处非阻塞但这一轮最好一起收紧的点：

- TCP RX 的生命周期语义还只覆盖 happy path
- validation 还没有把“宣称的外部兼容契约”本身钉成可回归检查项

---

## Findings

### R-001 `Declared KXemu compatibility baseline is still factually inconsistent`

- Severity: HIGH
- Section: `Summary / Constraints / Memory Map`
- Type: Spec Alignment
- Problem:
  `02_PLAN` 多处声明“Compatibility baseline = KXemu virt DTS”，并且写成“all MMIO sizes ... match this single reference”。但文档里的 PLIC guest-visible size 仍然写成 `0x400_0000`，而本地 `~/Emulator/KXemu/tests/riscv/opensbi/riscv64-virt.dts` 的 PLIC `reg` 明确是 `0x600000`。也就是说，当前不是“ACLINT 取代 CLINT 的有意差异”这么简单，而是 baseline 自己就没有对齐。
- Why it matters:
  这一轮已经把 compatibility baseline 当成正式外部契约来写入 `C-1`、Memory Map 和 Response Matrix。如果这里仍然不准确，那么后续实现出来的 guest-visible MMIO shape 仍然会继续漂移，review 也无法接受 “R-004 resolved” 这类结论。
- Recommendation:
  下一个 PLAN 必须二选一并全文统一：
  1. **真的匹配 KXemu virt DTS**：把 PLIC guest-visible size 改成 `0x600000`，并明确说明“按 M-006 以 ACLINT 取代原 CLINT”；
  2. **不再声称精确匹配 KXemu DTS**：把 baseline 改写成“qemu-/KXemu-like address layout with ACLINT substitution”，删掉“all MMIO sizes match”这类过度表述。

### R-002 `Bus→PLIC mechanism is still not a single finalized design`

- Severity: HIGH
- Section: `Architecture / API Surface / Bus::tick()`
- Type: API
- Problem:
  文档口头上说 `M-002` 已应用，`Device` trait 要保持 clean，不引入 device-specific 方法；但真正落到 `Bus::tick()` 时，又写成“find by name, downcast”，随后注释再补一句“requires `as_any_mut()`; alternative: store PLIC index at registration time; implementation will choose the cleanest approach”。这说明本轮最核心的 Bus/device contract 其实还没有完全定稿。
- Why it matters:
  这不是文档表述问题，而是直接影响实现 write-set 和 trait surface 的结构问题：
  - 如果走 `as_any_mut()`，那 `Device` trait 实际上新增了一个只为少数设备服务的逃生口；
  - 如果走 “store PLIC index”，那 Bus 的注册/持有模型又需要被正式写进最终 API。

  当前 reviewer 仍然看到了两套不同的落地路径，而不是一套可以直接照着实现的 final design。
- Recommendation:
  下一个 PLAN 应只保留一套机制。更推荐：
  - `Bus` 在注册时保存 PLIC 的 index / handle
  - `tick()` 直接用这个 index 通知 PLIC
  - `Device` trait 不增加 `as_any_mut()`

  如果 executor 仍坚持 downcast，就必须把 trait 变更正式写进 API Surface，并解释为什么这不违反 `M-002`。

### R-003 `TCP RX lifecycle is only specified for the happy path`

- Severity: MEDIUM
- Section: `Step 3 / Validation`
- Type: Validation
- Problem:
  `Uart::with_tcp()` 的伪代码目前是：
  - `bind(...).unwrap()`
  - 单次 `accept()`
  - 连接断开后线程结束

  也就是说，本轮虽然把 TCP backend 作为正式 Phase 4B 方案写进去了，但其生命周期语义仍然只覆盖“端口可绑定、客户端连接一次、连接不断开”的 happy path。Validation 也只覆盖了正常收包路径。
- Why it matters:
  现在 UART RX 不再是可选附属方案，而是本轮批准范围的一部分。若端口占用直接 panic、断开后无法重连，这些行为至少要在计划层面被定义，否则实现出来后会在日常 bring-up 中表现得非常脆弱。
- Recommendation:
  下一个 PLAN 至少要明确：
  - bind 失败时的行为
  - 连接断开后的行为
  - 是否支持重新连接

  并补一个 robustness validation，例如“port occupied”或“disconnect then reconnect”。

### R-004 `Validation still does not lock the declared external contract`

- Severity: MEDIUM
- Section: `Validation / Memory Map`
- Type: Validation
- Problem:
  这一版 validation 已经把功能路径写得比较完整，但仍然没有任何一项直接验证“本轮声称的 guest-visible contract”本身，例如：
  - MMIO base / size
  - UART IRQ source assignment
  - `timebase-frequency` 相关配置语义

  当前 `C-1` 的漂移已经说明，仅靠行为测试不足以防止外部接口继续失真。
- Why it matters:
  即使实现通过了功能测试，也仍然可能在 memory map / wiring 这类外部契约上悄悄跑偏；而这恰恰是后续 OS / firmware bring-up 最敏感的部分。
- Recommendation:
  下一个 PLAN 应补一类 config-level validation，显式检查注册到 Bus 的 base / size / irq_source 与本轮 baseline 一致；如果 `timebase-frequency` 只是文档契约，也应增加对应的说明性验证或检查项。

---

## Trade-off Advice

### TR-1 `Prefer a stored PLIC handle over trait-object downcast`

- Related Plan Item: `T-4`
- Topic: Clean Abstraction vs Runtime Flexibility
- Reviewer Position: Prefer Option A
- Advice:
  保留 “Bus 汇总 irq_line，再内部通知 PLIC” 这个总体方向，但把机制固定为 **registration-time 保存 PLIC index / handle**，不要把 `as_any_mut()` 作为默认落地方向。
- Rationale:
  这条路径更符合 `M-002`：Bus 可以持有少量 wiring-level knowledge，但 `Device` trait 本身不需要因为一个特殊设备而泄露 `Any`/downcast 逃生口。实现上也更直接，避免字符串查找和运行时 downcast。
- Required Action:
  下一个 PLAN 应正式选择这一方案，或明确说明为什么不选它。

### TR-2 `Keep TCP RX, but narrow the operational contract`

- Related Plan Item: `T-3`
- Topic: Simplicity vs Operational Robustness
- Reviewer Position: Prefer Option A
- Advice:
  TCP 作为 UART RX backend 的方向本身是对的，这一轮没必要再退回 stdin / PTY 讨论；但应把它收紧成一个明确的运行契约，而不是默认“网络线程自然会工作”。
- Rationale:
  现在最大的风险已经不是 architecture，而是 operational ambiguity。TCP 已经解决了 `xdb` stdin 冲突；剩下的问题是 bind、disconnect、reconnect 这些行为是否被定义。
- Required Action:
  下一个 PLAN 应保留 TCP backend，同时补齐 lifecycle 语义和一条 robustness validation。

---

## Positive Notes

- `02_PLAN` 这次真正解决了上一轮的 scope ambiguity，`G-3b` 已经不再是口头包含。
- PLIC 的 claimed-exclusion 语义这次是对的，`R-002` 的核心 correctness bug 已经修掉。
- `irq_state` + `sync_interrupts()` 的收口方式比之前干净，`step()` 顺序也写得足够明确。
- 用 TCP 替代 stdin 作为 UART RX backend，是当前 `xdb` 工作流下正确的方向。

---

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003
- R-004

### Trade-off Responses Required
- T-3
- T-4

### Ready for Implementation
- No
- Reason: the declared compatibility baseline is still wrong, and the Bus→PLIC mechanism is not yet a single finalized implementation contract.
