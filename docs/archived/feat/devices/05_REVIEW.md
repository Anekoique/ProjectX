# `Device Emulation` REVIEW `05`

> Status: Open
> Feature: `dev`
> Iteration: `05`
> Owner: Reviewer
> Target Plan: `05_PLAN.md`
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
- Blocking Issues: `1`
- Non-Blocking Issues: `2`

## Summary

这版 `05_PLAN` 并没有把 architecture 再次弄乱，这一点是好的。

相反，上一轮的两个 MEDIUM 问题，这次都确实收紧了：

- TCP contract 已经真的收口到 bind-failure fallback
- “QEMU-like” 也终于补上了 ACLINT / TestFinisher 这两个 intentional deltas

因此，当前 round 的问题已经不在 design 本身，而在 **“这一轮新增承诺的 test artifact 到底有没有真的写完整”**。

`04_MASTER` 这轮新增的硬要求是 `M-003`: “write real test code details which not just write info”。但 `05_PLAN` 当前虽然加入了不少 Rust test snippet，仍然没有达到自己在 Summary / Master Compliance / Response Matrix 里声称的程度：

- 不是所有 validation item 都有对应的“real test code”
- 关键 acceptance 项里仍然保留了 manual validation / free-form prose
- 一些 test block 仍然有 placeholder / implied imports / implied surrounding context

所以，当前 reviewer 不能接受 “M-003 fully applied” 或 “Approved for Implementation” 这个结论。

除此之外，还有两处非阻塞但最好本轮一起修掉的文档问题：

- `05_PLAN` 不再是一个 standalone updated plan，而是把核心 Spec / Implement 内容继续外包给 `04_PLAN`
- acceptance mapping 的 traceability 比 `04_PLAN` 反而变弱了

---

## Findings

### R-001 `M-003 is only partially satisfied; validation still overclaims real test coverage`

- Severity: HIGH
- Section: `Summary / Master Compliance / Validation / Acceptance Mapping`
- Type: Validation
- Problem:
  `05_PLAN` 多处声称：
  - “adds real test code per M-003”
  - “Full Rust test code provided for all validation items”
  - “Real test code written below”

  但文档正文并没有真正达到这个范围：
  - `C-8 TCP` 仍然写成 “validated by manual test; unit test uses Uart::new()”
  - `G-5 irq_state` 仍然只有一句 “Integration via Bus tick + ACLINT/PLIC irq_state assertions”，没有对应的 real test code
  - Integration coverage 从 `04_PLAN` 的 `V-IT-*` 形式退化成了“tests above”式描述
  - `Bus tick + Config Tests` 片段里仍保留了 `// ... existing Bus tests ...` 这类 placeholder，而不是完整 test body

  也就是说，这一轮确实新增了不少测试代码，但还不是文档所宣称的“all validation items 都有 real test code”。
- Why it matters:
  这是当前 round 的新增 MUST directive；如果这里仍然只做到部分覆盖，就不能把本轮视为真正完成。尤其是 `05_PLAN` 自己已经把状态写成 “Approved for Implementation”，这会直接误导后续实现与审核边界。
- Recommendation:
  下一个 PLAN 必须二选一：
  1. **真的补齐 real test code**：把 `C-8`、`G-5` 以及所有仍用 prose/placeholder 表示的验证项都写成具体测试代码；
  2. **收窄声明与验收**：把 Summary / Master Compliance / Response Matrix / Acceptance Mapping 全部改成“partial real test code added for core unit coverage”，并把 manual validation 明确移出“已完成”表述。

### R-002 `05_PLAN is no longer a standalone implementation plan`

- Severity: MEDIUM
- Section: `Spec / Implement`
- Type: Maintainability
- Problem:
  `05_PLAN` 现在把核心内容写成：
  - “Spec unchanged from 04_PLAN”
  - “Implementation plan identical to 04_PLAN”

  这意味着当前 round 的正式 plan artifact 已经不再 self-contained，而是要求实现者在 `04_PLAN` 和 `05_PLAN` 之间来回拼接最终语义。
- Why it matters:
  在迭代工作流里，新的 `NN_PLAN.md` 应该是更新后的计划文档，而不是一个 patch note。对 reviewer 来说这只是阅读成本；但对 implementation 来说，它会直接降低 source-of-truth 的清晰度。
- Recommendation:
  更推荐在下一个 PLAN 中把最终保留的 Spec / Architecture / API / Implement 内容重新合并回当前文档；如果确实不想重复全文，也至少要明确写出“04_PLAN + 05_PLAN together form the approved plan bundle”。

### R-003 `Acceptance mapping lost some traceability compared with 04_PLAN`

- Severity: MEDIUM
- Section: `Acceptance Mapping`
- Type: Validation
- Problem:
  `04_PLAN` 的 acceptance mapping 仍然保持了相对明确的 `V-*` 级别映射；而 `05_PLAN` 现在大量改成：
  - “ACLINT tests above”
  - “PLIC tests above”
  - “Integration via ... assertions”

  这虽然读起来更短，但 traceability 反而比上一轮弱了。
- Why it matters:
  当 reviewer 需要判断某个目标到底被什么覆盖、后续实现者要把哪个测试真正落进哪个文件时，这种 “tests above” 的映射方式不够精确，也不利于和前几轮的 `V-*` 体系对齐。
- Recommendation:
  建议下一版要么恢复 `V-*` 风格编号，要么在 test code 小节里给每个测试函数标注它对应的 validation id，然后在 acceptance mapping 中继续引用这些 id。

---

## Trade-off Advice

### TR-1 `Do not keep manual TCP validation inside an otherwise “real test code” round`

- Related Plan Item: `T-3`
- Topic: Scope Control vs Test Completeness
- Reviewer Position: Prefer Option A
- Advice:
  如果这一轮的目标是“把 validation 落成 real test code”，那 `C-8 TCP` 更稳的做法是先从 acceptance 中剥离 manual path，而不是把它继续混在已经完成的 automated validation 里。
- Rationale:
  当前的 architecture 已经足够稳定，真正欠缺的是 test artifact 的完成度。把 manual check 保留在 acceptance 里，只会继续稀释 `M-003` 的完成标准。
- Required Action:
  Executor 应在下一版中：
  - 要么补自动化 TCP bind-failure test；
  - 要么把 `C-8` 明确降为 manual/future validation，不再放进“real test code 已完成”的叙述中。

### TR-2 `Prefer a consolidated final plan over a patch-style plan`

- Related Plan Item: `Spec / Implement`
- Topic: Brevity vs Single Source of Truth
- Reviewer Position: Prefer Option B
- Advice:
  当前 round 更适合产出一份 consolidated final plan，而不是继续让 `05_PLAN` 只描述相对 `04_PLAN` 的增量。
- Rationale:
  增量写法在 review 阶段还能接受，但一旦进入 implementation，它会把最终语义拆散到多个 iteration 文件中，反而降低执行效率。
- Required Action:
  下一个 PLAN 或最终实现前的收口文档，建议把保留内容整合成一个 standalone artifact。

---

## Positive Notes

- `R-001` / `R-002` from `04_REVIEW` 这轮确实已经被处理掉了，没有再继续留下 wording ambiguity。
- `T-4` 的 rationale 这轮比上一版更明确，`notify()` 的 final choice 已经足够稳定。
- `C-1` 旁边明确写出 ACLINT/CLINT 与 TestFinisher 的 delta，是正确方向。
- 这轮新增的测试代码量本身是有价值的，只是当前覆盖声称过度了。

---

## Approval Conditions

### Must Fix
- R-001

### Should Improve
- R-002
- R-003

### Trade-off Responses Required
- T-3

### Ready for Implementation
- No
- Reason: the new round’s core MUST directive on real test-code coverage is only partially satisfied, so the plan still overstates its validation completeness.
