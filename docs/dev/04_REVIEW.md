# `Device Emulation` REVIEW `04`

> Status: Open
> Feature: `dev`
> Iteration: `04`
> Owner: Reviewer
> Target Plan: `04_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: `0`
- Non-Blocking Issues: `2`

## Summary

这版 `04_PLAN` 已经基本达到了 implementation-ready。

上一轮的两个 blocking issue，这一轮都已经实质收掉了：

- baseline 终于不再同时依赖 `KXemu DTS`、`def.hpp` 和 “QEMU-like” 三套口径，而是明确退回到 **xemu internal layout**
- Bus→PLIC 的最终接口也终于稳定成了 **`plic_idx + Device::notify()`**，并且没有再保留上一轮那些互斥草稿

此外，`mmio_regs!` 的语义也被明显纠正了，不再硬说成“统一抽象”，而是老实收口成 fixed-offset helper；这一点比 `03_PLAN` 明显更稳。

当前剩下的问题已经不是 architecture blocker，而是两处应在实现前顺手再收紧的文档/验收边界问题：

- TCP lifecycle 仍然有一小段“已写入 contract 但未纳入验收”的语义
- “QEMU-like” 这个表述现在只是 orientation label，但文档还可以更明确地写出它与官方 `virt` 的有意差异

整体判断：可以开始实现，但建议先把下面两点一起修到位。

---

## Findings

### R-001 `TCP lifecycle wording still slightly exceeds the validated scope`

- Severity: MEDIUM
- Section: `Summary / Review Adjustments / Constraints / Trade-offs / Acceptance Mapping`
- Type: Validation
- Problem:
  `04_PLAN` 已经比上一轮诚实很多，明确写了 “disconnect not validated this round”。但当前文档仍然同时存在两层说法：
  - Summary / Review Adjustments 说 “contract narrowed to validated scope only”
  - `C-8` / `T-3` / `I-7` 里仍然把 “disconnect → RX stops; no reconnect” 写成正式 contract，只是额外加了“本轮未验证”的注记

  也就是说，本轮实际上不是“只保留了已验证范围”，而是“保留了更大的 contract，但承认其中一部分未验证”。
- Why it matters:
  这已经不再是 blocking issue，但会继续影响 round 的验收边界。实现者和后续 reviewer 需要知道：本轮到底批准到 “bind-failure fallback” 为止，还是同时也接受了 disconnect/no-reconnect 语义只是暂时不测。
- Recommendation:
  更推荐二选一：
  1. **真正收窄 contract**：把 `C-8` / `T-3` / `I-7` 只保留到 bind-failure fallback；
  2. **保留当前 contract**：但把 Summary / Review Adjustments 的表述改成“contract is partially validated this round”，不要再写 “validated scope only”。

### R-002 `"QEMU-like" now works as orientation, but the intended deltas should be explicit`

- Severity: MEDIUM
- Section: `Summary / Constraints / Memory Map`
- Type: Spec Alignment
- Problem:
  `04_PLAN` 现在已经正确地把 baseline 收口成 `xemu internal layout`，这一点是对的。但文档仍然频繁使用 “QEMU-like” 作为方向标签，而官方 QEMU `virt` 文档当前明确列出的是：
  - `CLINT`
  - `PLIC`
  - `NS16550 UART`
  - `SiFive Test device`

  当前 xemu 计划中的默认 machine 则是：
  - `ACLINT` 替代 `CLINT`
  - `TestFinisher` 不进入默认 wiring，仅 test-only

  这不是错误，但现在还没有在文档中被明确列为 “intentional deltas from qemu virt orientation”。
- Why it matters:
  既然本轮已经不再声称 external compatibility，那最稳的写法就是把这些差异提前说清楚，避免后续再有人把 “QEMU-like” 理解成 “should match QEMU virt device set more closely than it actually does”。
- Recommendation:
  在下一版 PLAN 或实现说明中，建议在 `C-1` 或 Memory Map 旁边补一行简短说明：
  - “QEMU-like in address/layout shape only”
  - “Intentional deltas: ACLINT replaces CLINT; TestFinisher is test-only”

  这样当前定位就会完全清楚。

---

## Trade-off Advice

### TR-1 `Be explicit about whether TCP disconnect is approved behavior or future behavior`

- Related Plan Item: `T-3`
- Topic: Scope Control vs Behavioral Completeness
- Reviewer Position: Prefer Option A
- Advice:
  如果本轮不准备验证 disconnect / no-reconnect，就更推荐把这部分降为 future behavior note，而不是继续放在当前 round 的正式 contract 中。
- Rationale:
  现在 bind-failure fallback 已经足够支撑实现落地；继续把未验证语义写进正式 contract，只会让 acceptance boundary 再次变模糊。
- Required Action:
  Executor 可在下一版文档或实现说明中二选一：
  - 从当前 contract 中移除 disconnect/no-reconnect；
  - 或保留，但明确标注为 “documented future behavior, not accepted this round”。

### TR-2 `Keep notify() as the final API, but pair it with a short rationale note`

- Related Plan Item: `T-4`
- Topic: Clean Abstraction vs Pragmatic Wiring
- Reviewer Position: Keep as is with clarification
- Advice:
  `notify()` 这次已经足够收敛，不建议再回头改设计；只建议在实现说明里再补一句“why this one extra trait hook is preferred over Bus special-casing or downcast”。
- Rationale:
  当前 `notify()` 已经比前几轮清楚很多，继续换方案只会打乱已经稳定下来的 plan。剩下的只是把 rationale 再写短一点、硬一点，让后续实现和 review 都能直接复用。
- Required Action:
  保持 `notify()` 方案不变；在下一版文档或实现提交说明中补一条简短 rationale 即可。

---

## Positive Notes

- `04_PLAN` 终于把上两轮最核心的 plan-level ambiguity 清掉了。
- `Device::notify()` 现在是明确、统一、可执行的最终接口，不再有前后自相矛盾的问题。
- baseline 退回到 `xemu internal layout` 是正确收口，比继续拿外部引用做不完整背书稳得多。
- `mmio_regs!` 的定位这次明显更准确，和 `PLIC/UART` 仍然手写 decode 的现实不再冲突。

---

## Approval Conditions

### Must Fix
- None

### Should Improve
- R-001
- R-002

### Trade-off Responses Required
- None

### Ready for Implementation
- Yes
- Reason: previous blocking issues are resolved, and the remaining concerns are documentation/acceptance-boundary refinements rather than architecture blockers.
