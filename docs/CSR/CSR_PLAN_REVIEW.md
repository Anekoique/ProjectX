# CSR_PLAN 设计复评

> 针对 [CSR_PLAN.md](./CSR_PLAN.md) 的第三轮评审
> 评审时间：2026-03-21
> 目标：在开始实现前，确认 CSR subsystem 的分层、trap 边界和实现顺序足够干净、简洁、可扩展

## 结论

当前这版 [CSR_PLAN.md](./CSR_PLAN.md) 已经接近实现就绪。

相比前两版，几个最关键的问题已经处理掉了：

- `PendingTrap` 已经从 `XError` 通道中剥离
- `RVCore` / `CsrFile` 的职责边界已经清楚
- trap 改为写 `npc`，修复了 `pc/npc` 提交冲突
- `ebreak` 已改为 `Breakpoint` trap
- `TSR`、`TVM`、`mcounteren/scounteren` 已纳入访问规则
- `mret` / `sret` 的 `MPRV` 处理已补齐
- `mtvec/stvec` 已明确只支持 direct mode
- 中断采样点已经前移到 `step()` 之前

这意味着该设计可以进入实现阶段。

但从“代码要保持 clean, concise, elegant”的角度看，开始编码前仍建议先修掉下面两项高优先级设计缺口，否则实现很容易演变成一套不干净的双通道异常模型。

## 高优先级问题

### 1. trap / error 边界仍未完整定义

计划在 [CSR_PLAN.md](./CSR_PLAN.md) 中明确写了：

- 所有架构性 trap 都应走 `PendingTrap`
- `Err(XError)` 只保留给宿主 I/O、模拟器内部错误和实现 bug

这个原则本身是对的，但当前计划还没有把“现有代码里哪些 `XError` 需要迁移到 trap 通道”写清楚。

而当前 `xemu` 里，相关路径仍然大量依赖 `Err(XError)`：

- `dispatch()` 对未知指令直接返回 `InvalidInst`
- 各类 RV32/RV64/C 指令实现里仍有大量 `InvalidInst`
- `decode()` 会返回 `PatternError` / `ParseError` / `DecodeError`
- memory 访问仍返回 `BadAddress` / `AddrNotAligned`

如果这一层不先定义清楚，CSR subsystem 一开始实现出来就会变成：

- CSR / privileged path 走 `PendingTrap`
- 其余架构 fault 继续走 `Err(XError)`

这会导致执行模型一半是架构 trap，一半是宿主错误，后面会越来越难收敛。

建议在计划中补一小节，明确下面三件事：

1. 哪些 `XError` 继续保留为 emulator-internal error  
2. 哪些现有 `XError` 要在 Phase A/C 就地转换为架构 trap  
3. trap 转换发生在哪一层  

我建议的收口方式是：

- `fetch/decode/execute` 仍然可以临时返回内部错误
- 但在 `CPU::step()` 或 `RVCore` 边界统一把“架构性 fault”翻译成 `PendingTrap`
- 最终目标是让 `InvalidInst`、对齐错误、访问错误都不再直接冒泡到顶层 run loop

如果不先写清楚这个边界，实现过程中会不断出现“这个错误到底该 return Err 还是 raise_trap”的分叉，代码会变脏。

### 2. Phase D 说了“中断在 fetch 前采样”，但还没有把控制流写完整

现在计划已经正确地把中断采样点放到了 `step()` 里的 fetch 之前，这个方向是对的。

但文档还缺最后一步：  
如果在 fetch 前发现 pending interrupt，`step()` 应该怎么收尾。

当前 trap 提交通路仍然主要通过 execute loop 展示：

- `dispatch()`
- 检查 `pending_trap`
- `commit_pending_trap()`
- `pc = npc`

这对同步异常是成立的，但对“fetch 之前就发现的异步中断”还不够完整。  
如果这个控制流不提前写死，最常见的错误实现会是：

- 先 set `pending_trap`
- 然后仍然继续 fetch/decode 一条本不该执行的指令

建议在计划中直接补一段 `step()` 级伪代码，明确：

```rust
fn step(&mut self) -> XResult {
    if let Some(cause) = self.check_pending_interrupt() {
        self.raise_trap(TrapCause::Interrupt(cause), 0);
        self.commit_pending_trap();
        self.pc = self.npc;
        return Ok(());
    }

    let inst = self.fetch()?;
    let decoded = self.decode(inst)?;
    self.execute(decoded)
}
```

重点不是这段代码本身，而是要把规则写清楚：

- 异步中断在指令边界进入
- 命中后本轮不再 fetch/decode/execute
- trap 仍然复用同一个 `commit_pending_trap()` 提交通路

这样实现会保持单一控制流，而不会出现第二套“中断专用 trap 入口”。

## 中优先级建议

### 3. `csr_read()` 里的 `read_side_effects()` 建议删掉

当前计划里：

- `write_side_effects()` 有清晰的边界定义
- 但 `csr_read()` 伪代码里仍然有一行 `self.read_side_effects(addr)`
- 文档后文并没有定义这个函数，也没有列出任何 Phase 2 需要的 CSR 读副作用

这会让接口表面上看起来对称，但实际上只是引入一个没有语义支撑的空扩展点。

为了保持代码简洁，我建议：

- Phase 2 只保留 `write_side_effects()`
- `read_side_effects()` 先不要引入
- 以后真的出现需要读副作用的 CSR，再按真实需求补进去

不要为了“形式对称”先加一个空 hook。

### 4. `read_masked()` / `write_masked()` 应该尽量收紧为内部接口

计划当前同时保留了四组访问方式：

- `RVCore::csr_read/csr_write`  
- `CsrFile::read_with_desc/write_with_desc`  
- `CsrFile::read_masked/write_masked`  
- `CsrFile::get/set(CsrAddr)`  

从功能上说这是够用的，但从边界清晰度看，真正应该鼓励使用的只有两类：

- 普通指令路径：`RVCore::csr_read/csr_write`
- trap handler 等 trusted path：`get/set(CsrAddr)` 或 `read_with_desc/write_with_desc`

`read_masked/write_masked` 这种“带 lookup 的原始访问”很容易在后续代码里被误用，绕开 `RVCore` 的 policy 层。

建议：

- 至少把它们明确标成 internal helper
- 更进一步的话，可以只在 `csr/mod.rs` 内使用，不暴露为外部常用接口

这不是 correctness bug，但对长期保持代码干净很重要。

## 建议保留的设计

下面这些设计我认为已经收敛得比较好，不建议再改大方向：

- `[Word; 4096]` 作为 CSR 存储底座
- `sstatus` / `sie` / `sip` 作为 `mstatus` / `mie` / `mip` 的 masked view
- `CsrFile` 只负责存储和 descriptor，`RVCore` 负责策略和副作用
- `AccessRule` 进入 `CsrDesc`，用声明式规则替代魔法地址判断
- `write_side_effects()` 保留在 `RVCore`，而不是把 hook 塞进 `CsrDesc`
- `csr_table!` 同时生成 `CsrAddr` 和 descriptor lookup，避免表和枚举漂移
- `mtvec/stvec` 当前阶段强制 direct mode

这套组合已经足够适合 `xemu` 当前阶段，复杂度也控制得住。

## 推荐的实现顺序

如果目标是“边做边保持代码漂亮”，我建议按下面顺序推进：

1. 先实现 Phase A，但同时补上 trap/error 边界说明
2. 再做 Phase B，先把 `CsrFile` 和 `MStatus` 打稳
3. Phase C 只让 CSR / privileged 指令接入 policy 层，不要顺手扩散别的抽象
4. 在进入 Phase D 前，先把 `step()` 的中断控制流写成明确伪代码
5. Phase E 做 counters 时，优先保持 alias/访问规则简单，不要引入额外状态机

一句话总结就是：

- trap 通道先收口
- CSR 分层保持单一职责
- 不提前引入空 hook 和重复入口

这样实现出来的代码会比较接近你想要的风格：clean, concise, elegant。

## 最终判断

当前 `CSR_PLAN` 已经可以开工。

如果只在实现前再补两件事，这份计划就足够稳：

1. 把 trap / error 的迁移边界写清楚  
2. 把 step 级的异步中断控制流写完整

除此之外，不建议再做大的结构调整。
