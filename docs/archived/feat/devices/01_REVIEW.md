# `Device Emulation` REVIEW `01`

> Status: Open
> Feature: `dev`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
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

这版 `01_PLAN` 相比 `00_PLAN` 进步很明显。

几个上一轮的关键问题，executor 确实已经正面回应了：

- `00_REVIEW` / `00_MASTER` 的 Response Matrix 补齐了
- UART 不再直接抢 `xdb` 的 stdin
- `DeviceIrq` 的 one-shot 思路被移除，开始转向 level-triggered 设计
- 函数级伪代码和 wiring 细节也比上一轮完整得多

这些都是实质性改进。

但当前文档还不能进入实现，原因有两点：

- 这一轮到底批准到 **Phase 4A** 还是同时批准 **Phase 4B**，边界仍然不清楚
- 当前 PLIC 伪代码会在 source 仍处于 claimed 状态时就重新 pend level interrupt

除此之外，还有两个非阻塞但本轮最好一起收紧的问题：

- 文档里仍然保留了多套互斥的 device→PLIC 设计草稿，final architecture 不够收敛
- “qemu-virt DTS conventions” 这个兼容目标仍然说得过宽，当前值更像是混合了不同参考实现

---

## Findings

### R-001 `Round scope is still ambiguous between Phase 4A and Phase 4B`

- Severity: HIGH
- Section: `Summary / Goals / Implementation Plan / Acceptance Mapping`
- Type: Flow
- Problem:
  `01_PLAN` 一方面把 UART 拆成 `G-3a` 和 `G-3b`，并在 Summary / Response Matrix 中宣称 “R-001 resolved”；另一方面，本轮真正写全的实现与验收却主要停留在 Phase 4A。Acceptance Mapping 只映射了 `G-3a`，没有 `G-3b`；Validation 也没有完整的 TCP RX 端到端 acceptance。
- Why it matters:
  按新的工作流，`NN_PLAN.md` 是这一轮是否可以开工的批准单元。如果本轮 scope 包含 `G-3b`，那它的验收还不完整；如果不包含，那 `R-001 resolved` 和 Summary 里的表述就过度了。当前 reviewer 无法据此判断“这一轮到底批准到哪里”。
- Recommendation:
  下一个 PLAN 必须二选一：
  1. **严格收口为 Phase 4A-only**：删除 `G-3b` 出本轮 scope，把 TCP RX 明确移到 `02_PLAN.md`；
  2. **保留 G-3b 在本轮**：补齐 TCP RX 的 integration validation、acceptance mapping、and explicit done criteria。

### R-002 `PLIC update_pending() re-pends claimed level interrupts too early`

- Severity: HIGH
- Section: `Step 2 / Invariants / State Transition`
- Type: Correctness
- Problem:
  现在的 `update_pending(device_lines)` 逻辑是：只要 line 高，就直接 `self.pending |= bit`；line 低时才在“not claimed”条件下清 pending。这样一来，source 被 `claim()` 取走后，只要 line 仍为高，下一次 `tick()` 就会再次进入 pending，即使 guest 还没有 `complete()`。
- Why it matters:
  这会破坏 claim/complete 的在服役语义。对 level-triggered source，正确行为应该是“**complete 之后** 如果 line 仍高，再重新变成 pending”，而不是“claimed 期间每个 tick 都重新 pend”。否则会导致重复外部中断、spurious re-entry，甚至形成 interrupt storm。
- Recommendation:
  `update_pending()` 必须把 “currently claimed” 视为一个禁止 re-pend 的状态。建议改成：
  - line 高且 **not claimed** → set pending
  - line 低且 **not claimed** → clear pending
  - claimed → 保持 non-pending，直到 `complete()` 后下一次 tick 再根据 line 决定是否重新 pend

### R-003 `The final device→PLIC architecture is still not fully converged`

- Severity: MEDIUM
- Section: `Architecture / Data Structure / API Surface`
- Type: Maintainability
- Problem:
  `01_PLAN` 的最终文档里仍然同时保留了多套互斥方案：
  - `PlicSource / sources` 的 source registry 方案
  - `find_plic_mut() / set_device_lines()` 方案
  - `Bus.device_lines + post_tick()` 的 final 方案

  文档中甚至还保留了 “Wait — this re-introduces the one-shot problem. Let me reconsider.” 这种中间思考痕迹。
- Why it matters:
  这不是单纯的文风问题。它会直接影响 executor 对最终 write-set 和接口面的判断，尤其是 `Plic` 到底是否需要保存 `sources`、`Bus` 是否需要 special-case PLIC、`post_tick()` 是否是最终正式接口。这类歧义非常容易在实现时演化成“边写边改设计”。
- Recommendation:
  下一个 PLAN 应只保留一套最终设计。若最终选择的是 `Bus.device_lines + post_tick()`：
  - 删除 `PlicSource` / `sources`
  - 删除 `find_plic_mut()` / `set_device_lines()` 方案
  - 删除所有 “alternative / wait / reconsider” 的中间讨论

### R-004 `The claimed qemu-virt compatibility target is still under-specified`

- Severity: MEDIUM
- Section: `Constraints / Memory Map / Trade-offs`
- Type: Spec Alignment
- Problem:
  文档现在写的是“guest-visible sizes follow QEMU virt DTS conventions”，并把 `timebase-frequency = 10_000_000` 也一起固化了。但本地参考并不统一：
  - `Nemu-rust` DTS：PLIC `0x4000000`，`timebase-frequency = 250000`
  - `KXemu` / OpenSBI 风格 DTS：UART `0x100`，CLINT `0x10000`，PLIC `0x600000`，`timebase-frequency = 10_000_000`
  - 官方 QEMU `virt` 文档强调的是：guest 应依赖 DTB 发现设备，而不是硬编码假设
- Why it matters:
  当前 plan 实际上是在混用不同参考实现的外部契约。只要未来进入 DT / OpenSBI / SBI handoff，这些 externally visible values 就必须有一个明确 source of truth，否则后面还会再起一轮 compatibility drift。
- Recommendation:
  下一个 PLAN 应明确指定一个 compatibility baseline，例如：
  - “match QEMU virt generated DTB”
  - 或 “match local KXemu virt DTS”
  - 或 “match Nemu-rust DTS”

  然后把 UART/PLIC/CLINT 的 `reg` size、`interrupts-extended`、`timebase-frequency` 全部按同一个 baseline 记录。

---

## Trade-off Advice

### TR-1 `Keep 01 scoped to Phase 4A unless TCP RX is fully accepted`

- Related Plan Item: `T-3`
- Topic: Scope Control vs Feature Completeness
- Reviewer Position: Prefer Option A
- Advice:
  更稳的做法是把 `01_PLAN` 严格收口为 Phase 4A：CLINT + PLIC + UART TX + TestFinisher，TCP RX 单独放进 `02_PLAN.md`。
- Rationale:
  当前文档已经把 TX-only 路径写得比较完整，而 TCP RX 还处在“计划内但未形成完整 acceptance”的状态。把它拆出去，会让这一轮更容易得到真正可执行的批准。
- Required Action:
  Executor 应在下一个 PLAN 中明确：
  - `01` 是否只批准到 Phase 4A
  - 若不是，则必须补齐 `G-3b` 的验收闭环

### TR-2 `Prefer the Bus.device_lines + post_tick design, but make it the only one`

- Related Plan Item: `T-4`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A
- Advice:
  在当前 Bus ownership 模型下，`Bus.device_lines + post_tick()` 是合理的最终方向，但前提是它必须成为文档里唯一保留的 device→PLIC 设计。
- Rationale:
  相比 cross-device references 或 PLIC 自己维护 source registry，Bus 汇总 line state 更贴合现在的 `Bus { ram, mmio }` 结构，也更容易落地到现有代码上。当前问题不在于这个选择本身，而在于 plan 里还同时保留了别的互斥方案。
- Required Action:
  下一版只保留这一套方案，并删掉所有替代设计和中间思考痕迹。

---

## Positive Notes

- `01_PLAN` 已经开始认真处理上一轮 review / master 的约束，过程合规性明显比 `00_PLAN` 好。
- 对 UART stdin conflict 的修正方向是对的，至少不再让 `xdb` 和 guest console 正面抢输入。
- Level-triggered 设计方向也比 `DeviceIrq` one-shot 更接近正确语义。
- 详细的函数级伪代码已经足以支撑下一轮做真正的 implementation-ready 收敛。

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
- Reason: the approved unit of scope is still ambiguous, and the current PLIC level-trigger pseudocode is still incorrect for claimed sources.
