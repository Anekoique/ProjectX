# `Device Emulation` REVIEW `00`

> Status: Open
> Feature: `dev`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
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

这版 `00_PLAN` 比上一轮我看到的草案明显更稳。

几项关键改动已经到位：

- `NG-1` 明确把 OpenSBI / DT / SBI handoff 排除在本轮之外，范围比之前清楚
- `I-2` 已经把 `bus.tick() -> sync_external_interrupts() -> check_pending_interrupts()` 的顺序写死
- `Device::read(&mut self, ...)` 和 `Device::tick()` 也已经进入结构定义

这些调整说明 executor 已经把前一轮最明显的结构问题收紧了。

但当前方案仍然不能直接进入实现，原因不是“风格还可以再优化”，而是还有两处会直接影响可实现性的 blocking issue：

- UART RX backend 仍然和当前 `xdb` 的 stdin 模型冲突
- `DeviceIrq` 仍然按一次性事件建模，UART RX 会有丢中断风险

除此之外，还有两处非阻塞但最好本轮一起收口的点：

- 计划口头上追求 qemu-virt 兼容，但 guest-visible MMIO shape 还没有完全对齐
- 验证清单还没有覆盖本轮明确承认的高风险 trade-off

---

## Findings

### R-001 `UART RX backend still conflicts with xdb`

- Severity: HIGH
- Section: `Trade-offs / Implementation Plan / Validation`
- Type: Flow
- Problem:
  `T-3` 明确写了“Direct stdin ... conflicts with xdb”，但 `G-3` 和 Step 3 仍然把 UART RX 定义成“background stdin thread -> rx_buf -> rx_fifo”的默认实现路径。也就是说，这个冲突当前不是被规避了，而是被记录后继续保留在主方案里。
- Why it matters:
  现在 `make run` 的默认运行路径就是 `xdb` 交互模式。只要 UART RX 继续直接占用宿主 `stdin`，调试器命令输入和 guest 串口输入就会互相抢流，方案在当前主工作流里不可安全落地。
- Recommendation:
  下一个 PLAN 必须二选一：
  1. 把 `G-3` 收缩为 TX-only，把 RX 明确延期到下一轮；
  2. 保留 RX，但把 backend 改成 `TCP / PTY / file-backed input` 等不占用 `xdb` stdin 的方案。

### R-002 `DeviceIrq is still modeled as an event, not an asserted line`

- Severity: HIGH
- Section: `Architecture / Invariants / Step 2 / Step 3`
- Type: Correctness
- Problem:
  计划当前仍然定义“devices set `1 << source_id`, PLIC reads it in `tick()`”，并且 `I-3` 继续强调“claim clears pending bit”。这套语义对一次性事件可以工作，但对 UART RX 不够，因为 UART RX 的“有数据可读”本质上是 level-style condition，不是单次 edge/event。
- Why it matters:
  如果 guest claim 了 UART 中断，但 FIFO 里还有未读数据，而设备侧没有新的“事件”发生，那么中断不会被重新挂起。结果是 claim/complete 周期和串口可读状态脱钩，后面会出现难查的丢输入问题。
- Recommendation:
  下一个 PLAN 应把 `DeviceIrq` 明确改成“当前被设备拉高的 IRQ line”，或者让 PLIC 每次 `tick()` 从当前设备状态重算可断言源，而不是消费一次性事件位。同时把 invariant 和 validation 一起更新，新增“部分读取后仍可再次 claim”的测试。

### R-003 `Guest-visible MMIO shape is not fully aligned with the stated qemu-virt target`

- Severity: MEDIUM
- Section: `Constraints / Memory Map / API Surface`
- Type: Spec Alignment
- Problem:
  `C-1` 写的是“follow QEMU virt machine layout for future DT/OpenSBI compatibility”，但当前 memory map 仍然把 UART 暴露成 `8 B`、PLIC 暴露成 `0x0400_0000`。而本地 `KXemu` / `Nemu-rust` 的 virt DTS 都把 UART 暴露为 `0x100`，PLIC 为 `0x600000` 量级窗口；官方 QEMU `virt` 文档也明确说明 guest 应通过 DTB 发现设备地址和中断信息。
- Why it matters:
  如果计划的目标真的是“未来 DT / OpenSBI 兼容”，那么 guest-visible region shape 也是接口的一部分，不只是 base address。现在不锁定，后续进入 firmware / DT 阶段时还要再做一次兼容层或文档修正。
- Recommendation:
  下一个 PLAN 应明确区分：
  - guest-visible region size
  - emulator internal decoded register subset

  并优先把 guest-visible shape 调整到 qemu-virt 兼容值。

### R-004 `Validation does not yet cover the plan's explicit high-risk choices`

- Severity: MEDIUM
- Section: `Validation / Trade-offs`
- Type: Validation
- Problem:
  当前 validation 覆盖了寄存器和中断主路径，但没有覆盖这轮计划里自己承认的高风险决策：
  - UART backend 与 `xdb` 的共存策略
  - UART FIFO 未清空时的重复 claim/reassert 行为
  - host wall clock 作为 `mtime` 时，debugger pause 下的定时器语义
- Why it matters:
  如果这些点不进入 validation，本轮最可能出问题的地方会直接漏检，review 里指出的风险也无法在实现阶段被快速收敛。
- Recommendation:
  下一个 PLAN 应补至少 3 个验证项：
  - debugger-safe UART mode 的集成验证
  - UART FIFO 部分读取后的重复中断验证
  - host-time CLINT 在 pause/batch 模式下的行为验证或语义说明

---

## Trade-off Advice

### TR-1 `UART RX backend choice`

- Related Plan Item: `T-3`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  对本轮来说，更稳的选择是“不要让 UART RX 直接占用宿主 stdin”。
- Rationale:
  你当前已经有 `xdb` 作为默认交互入口。继续绑定 stdin 的收益只是实现快一点，但代价是直接破坏主工作流。相比之下，`TCP / PTY` 虽然多一点接线复杂度，但能把设备 bring-up 和 debugger 交互解耦。
- Required Action:
  Executor 应在下一个 PLAN 中改成非-stdin backend，或者明确把 RX 延期，只保留 TX。

### TR-2 `CLINT time source under interactive debugging`

- Related Plan Item: `T-2`
- Topic: Performance vs Simplicity
- Reviewer Position: Need More Justification
- Advice:
  如果继续选 host wall clock，就必须把 debugger pause 时 timer 是否继续前进写成显式语义；否则更建议改成 instruction/cycle-driven virtual time。
- Rationale:
  当前环境不是纯 batch simulator，而是带交互式 `xdb` 的调试器。timer 在 pause 期间如何演化，会直接影响调试可重复性和后续中断行为。
- Required Action:
  Executor 在下一个 PLAN 中必须补充这条语义说明，并把对应 validation 补齐；如果做不到，就应改用 virtual time。

---

## Positive Notes

- 这版计划已经把 Phase 4 和 OpenSBI / DT / SBI handoff 做了边界切分，范围比之前清楚。
- `step()` 前半段顺序现在写得足够明确，这一点明显比旧草案稳。
- `Device::read(&mut self)`、`Device::tick()`、`ExtIp` / `DeviceIrq` 这些接口面已经压到了一个相对干净的最小集合。

---

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003
- R-004

### Trade-off Responses Required
- T-2
- T-3

### Ready for Implementation
- No
- Reason: UART backend conflict and UART/PLIC interrupt semantics are still blocking issues in the current round.
