# `Device Emulation` REVIEW `03`

> Status: Open
> Feature: `dev`
> Iteration: `03`
> Owner: Reviewer
> Target Plan: `03_PLAN.md`
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

这版 `03_PLAN` 继续往 implementation-ready 靠近了。

相比 `02_PLAN`，这轮有几处是真正有效的改进：

- 不再使用 downcast，开始把 Bus→PLIC 路径往更明确的 wiring 收口
- TCP RX 的运行语义终于开始显式定义，不再只有 happy path
- config-level validation 被补进来了，说明 executor 已经意识到“外部契约”本身也需要检查
- TestFinisher 是否进默认 machine，也终于被写成了明确选择

但当前文档仍然不能批准进入实现，原因依旧是“核心契约还没有真正写成一个可执行的 final document”。

这一轮最关键的两个 blocking issue 是：

- 兼容性 baseline 仍然同时依赖 `KXemu DTS`、`KXemu def.hpp` 和 “QEMU-like” 三套口径
- Bus→PLIC 的最终 API 仍然在同一份文档里来回切换，最后实际落成的是 `plic_idx + Device::notify()`，但文档前半段却仍然声称“stored plic_idx only / no extra trait hook / no alternative traces”

另外两处非阻塞问题也值得这一轮一起收紧：

- TCP lifecycle 里“disconnect terminal / no reconnect”还没有进入真正的 validation
- `mmio_regs!` 的收益被写得比实际更大，当前更像是局部 helper，而不是已经统一掉设备寄存器抽象

---

## Findings

### R-001 `Compatibility baseline is still split across conflicting sources`

- Severity: HIGH
- Section: `Review Adjustments / Constraints / Memory Map / Validation`
- Type: Spec Alignment
- Problem:
  `03_PLAN` 现在把 `R-001` 标成 resolved，并把 `C-1` 改写成 “QEMU-like address layout, verified against KXemu def.hpp”。但当前本地参考本身仍然冲突：
  - `~/Emulator/KXemu/include/cpu/riscv/def.hpp` 给出的 PLIC size 是 `0x0400_0000`
  - `~/Emulator/KXemu/tests/riscv/opensbi/riscv64-virt.dts` 给出的 guest-visible PLIC `reg` size 是 `0x600000`
  - 官方 QEMU `virt` 文档强调 guest 应通过 DTB 发现设备配置

  这意味着当前 plan 仍然没有选定**一个**外部 source of truth，而是在内部实现常量、局部 DTS、和 “QEMU-like” 叙述之间切换。
- Why it matters:
  本轮新增的 `V-CF-1..4` 只能锁住 xemu 自己注册到 Bus 的常量，不能证明“guest-visible contract”已经和被引用的兼容目标对齐。也就是说，`R-001 resolved` 这个结论现在仍然站不住。
- Recommendation:
  下一个 PLAN 应明确二选一：
  1. 以 **当前 xemu 默认 machine layout** 作为 source of truth，并把它描述成“QEMU-like internal compatibility target”，不再借 `KXemu DTS` 为其背书；
  2. 以 **某个外部 guest-visible reference** 为 baseline，例如 KXemu DTS / QEMU DTB shape，并把所有有意偏离点单独列出来，例如 ACLINT 替代 CLINT、TestFinisher 不进默认 machine。

  在这之前，不应继续把 `def.hpp` 当作“已经解决 guest-visible baseline”的证据。

### R-002 `Bus→PLIC design is still internally inconsistent`

- Severity: HIGH
- Section: `Summary / Changes / Architecture / API Surface / Trade-offs`
- Type: API
- Problem:
  文档前半段多次声称：
  - “Bus→PLIC wiring via stored PLIC index”
  - “No device-specific methods on Device trait”
  - “Single finalized design”
  - “All alternative / TBD design traces removed”

  但真正到 Architecture 段时，文档仍然依次出现了：
  - `post_tick`
  - reserved PLIC offset / internal doorbell
  - Bus 单独持有 PLIC reference
  - helper 写入内部寄存器
  - 最终落到 `Device::notify()`

  最后真正的 API 其实是 `plic_idx + Device::notify()`，而不是前面声称的 “stored plic_idx only”。
- Why it matters:
  这不是措辞问题，而是当前 round 最核心的 bus/device contract 仍然没有被稳定表达出来。实现者现在看到的是：
  - Response Matrix 说一个版本
  - Architecture 中途推翻了好几次
  - API Surface 和 `Bus::tick()` 最后又落成另一个版本

  这正是上一轮 review 明确要求删除的 ambiguity。
- Recommendation:
  下一个 PLAN 必须只保留一套 Bus→PLIC 设计，并把整份文档改写到一致：
  - 如果最终选择是 `notify()`，那就明确写成“final API includes generic `Device::notify()` hook”，不要再写 “stored plic_idx only / no extra trait hook / no alternative traces”；
  - 如果最终不接受 `notify()`，那就把它从 trait 和所有伪代码里删掉，并给出唯一的替代机制。

### R-003 `TCP lifecycle is still only partially validated`

- Severity: MEDIUM
- Section: `Review Adjustments / Changes / Validation / Acceptance Mapping`
- Type: Validation
- Problem:
  `03_PLAN` 声称：
  - disconnect-terminal 语义已定义
  - `V-F-5`, `V-F-6` 已补上 bind / disconnect robustness validation

  但实际 validation 里：
  - `V-F-5` 是 “ACLINT mtime write ignored”
  - `V-F-6` 只覆盖 “UART TCP bind failure → TX-only”
  - 并没有任何 disconnect / no-reconnect 的验证项

  同时，`C-8 TCP lifecycle` 的 acceptance mapping 也只映射到 `V-F-6`。
- Why it matters:
  当前被正式写进 contract 的生命周期语义，比 validation 真正覆盖的内容更多。也就是说，这一轮虽然把语义写出来了，但还没有把它们真正纳入验收闭环。
- Recommendation:
  下一个 PLAN 应二选一：
  1. 增加 disconnect / no-reconnect validation；
  2. 收窄 `C-8` 和 Response Matrix，只保留 “bind failure fallback” 作为本轮已验收范围。

### R-004 `mmio_regs! is useful, but its current scope is overstated`

- Severity: MEDIUM
- Section: `Master Compliance / Changes / Data Structure / Trade-offs`
- Type: Maintainability
- Problem:
  文档当前多处把 `mmio_regs!` 描述成：
  - “replaces per-device register enums”
  - “unified declarative approach”

  但 plan 自己后面马上又说明：
  - ACLINT / TestFinisher 使用 `mmio_regs!`
  - PLIC / UART 仍然手写 decode

  这本身是合理的工程选择，但与前面的表述并不一致。
- Why it matters:
  这会让 reviewer 很难判断 `M-003` 到底是“已经形成统一抽象”，还是“只给固定-offset 设备提供了一个小型 helper”。两者都可以接受，但不能混着写。
- Recommendation:
  下一个 PLAN 应把这部分降格为更准确的表述，例如：
  - `mmio_regs!` 是 fixed-offset MMIO helper
  - PLIC / UART 保持手写 decode 是刻意保留，而不是暂时遗漏

  如果 executor 想继续宣称 “统一抽象”，那就需要更清楚地定义统一到哪一层。

---

## Trade-off Advice

### TR-1 `Narrow TCP lifecycle to what the round can actually validate`

- Related Plan Item: `T-3`
- Topic: Robustness vs Scope Control
- Reviewer Position: Prefer Option A
- Advice:
  TCP backend 方向不需要回退，但 lifecycle contract 应该只批准到当前能验证的部分。
- Rationale:
  现在 bind-failure fallback 已经写清楚了，但 disconnect / no-reconnect 还没有进入 validation。把未验证语义也一起当成“resolved”，只会继续放大文档和实现之间的偏差。
- Required Action:
  Executor 应在下一个 PLAN 中补齐 disconnect validation，或者把本轮 TCP contract 收窄为 “bind failure fallback + single accept happy path”。

### TR-2 `If notify() is the final abstraction, own it explicitly`

- Related Plan Item: `T-4`
- Topic: Clean Abstraction vs Pragmatic Wiring
- Reviewer Position: Need More Justification
- Advice:
  `notify()` 不是不能选，但如果要选，就应把它明确当作 Bus-level trait hook，而不是继续包装成“其实没有新增 trait surface”。
- Rationale:
  当前真正的问题已经不是 downcast，而是 plan 还在试图同时保住两套叙述：
  - “Device trait 没有新增特殊接口”
  - “实际新增了 notify()”

  reviewer 更希望看到明确、可执行的最终立场，而不是继续模糊这条边界。
- Required Action:
  下一个 PLAN 应明确接受或拒绝 `notify()`，并据此统一 Summary / Architecture / API Surface / Trade-offs / Response Matrix。

---

## Positive Notes

- 这一轮确实继续向 implementation-ready 推进了，尤其是 Bus wiring 和 TCP backend 都比 `02_PLAN` 更接近可落地状态。
- `V-CF-1..4` 这个方向是对的，至少开始把“外部契约”从纯口头描述往可检查项推进。
- 把 TestFinisher 是否进入默认 machine 写成显式选择，比之前模糊挂着更好。
- `plic_idx` 本身仍然比 downcast 更接近正确方向。

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
- Reason: the compatibility baseline is still not anchored to one external contract, and the Bus→PLIC API is still described inconsistently inside the same plan.
