# XError -> Trap 收口方案

> 目标：修复当前 `XError` 与 `raise_trap()` 双通道并存的问题
> 约束：少改代码，不破坏当前 `step -> retire -> commit_trap` 框架
> 核心思路：**把所有 guest-visible trap 统一编码成一个带 `cause/tval` 的 `XError` 变体，最后在 `step()` 统一处理**

## 结论

我认可这个方向，而且它比上一版 `TrapTaken` 更好。

真正优雅的收口方式，不是：

- 在很多调用点手写 `match`
- 或者先 `raise_trap()`，再用一个空的 sentinel error 提前返回

而是：

- **所有同步 trap 都返回 `Err(XError::Trap(...))`**
- **只有 `step()` 负责把这个 trap 放进 `pending_trap`，然后走现有 `retire()`**

这样：

- trap payload 和控制流合一
- 指令 handler 仍然可以继续用 `?`
- 不会引入一堆重复桥接代码

## 推荐的数据结构

这里我建议把你的想法具体化成一个**专用变体**，而不是给所有 `XError` 统一挂可选字段。

更好的形式是：

```rust
pub enum XError {
    Trap(PendingTrap),

    BadAddress,
    AddrNotAligned,
    PatternError,
    ParseError,
    DecodeError,
    InvalidInst,
    InvalidReg,
    FailedToRead,
    FailedToWrite,
    Unimplemented,
}
```

或者等价地：

```rust
pub enum XError {
    Trap {
        cause: TrapCause,
        tval: Word,
    },
    ...
}
```

两种形式都可以，但我更偏向：

```rust
XError::Trap(PendingTrap)
```

原因是：

- 你已经有 [`PendingTrap`](../../xemu/xcore/src/cpu/riscv/trap/cause.rs)
- 不需要再发明一套重复的 trap payload struct
- `pending_trap` 和 `XError::Trap` 承载的是同一种数据，只是处于不同阶段

## 这个设计里各对象的职责

这一点必须说清楚，不然又会退回双通道混乱。

### 1. `Err(XError::Trap(...))`

表示：

- 一个**同步架构 trap**
- 已经确定了 `cause` 和 `tval`
- 还没有进入 retirement/commit 阶段
- 当前执行路径应该立刻停止，并回到 `step()`

换句话说，它是：

- **trap 的传输形态**

### 2. `pending_trap`

表示：

- trap 已经进入 core 的 retirement 阶段
- 下一步会被 `retire()` / `commit_trap()` 消费

换句话说，它是：

- **trap 的提交队列形态**

### 3. 其他 `XError`

继续表示：

- 真正的模拟器内部错误
- 宿主 I/O 问题
- 当前实现 bug

也就是说：

- `XError::Trap(...)` 是 guest-visible architectural event
- 其他 `XError` 才是 host/internal error

## 为什么这比 `TrapTaken` 更好

`TrapTaken` 的问题是：

- 它只有控制流，没有 trap payload
- payload 还得靠 side effect 预先写进 `pending_trap`
- 于是“trap 数据”和“控制流停止”仍然是两套机制

而 `XError::Trap(PendingTrap)` 把这两件事合在一起了：

- 既携带 `cause/tval`
- 又天然通过 `?` 回到上层

这样同步 trap 的执行模型会更干净：

1. 深层代码构造 `Err(XError::Trap(...))`
2. `?` 一路向上传播
3. `step()` 统一接住
4. `step()` 把它塞进 `pending_trap`
5. `retire()` / `commit_trap()` 按现有逻辑提交

## 推荐的辅助接口

### 最小 helper

```rust
impl RVCore {
    #[inline(always)]
    fn trap<T>(&self, cause: TrapCause, tval: Word) -> XResult<T> {
        Err(XError::Trap(PendingTrap { cause, tval }))
    }
}
```

这一个 helper 就足够让大量同步 trap 代码变干净：

```rust
return self.trap(TrapCause::Exception(Exception::IllegalInstruction), raw as Word);
```

而不再是：

```rust
self.raise_trap(...);
return Ok(());
```

### memory error -> trap 的统一映射 helper

仍然需要一个很薄的 helper，但它只出现在少数 façade，而不是散落在所有 handler：

```rust
enum MemTrapKind {
    Fetch,
    Load,
    StoreAmo,
}

impl RVCore {
    #[inline(always)]
    fn trap_mem<T>(&self, err: XError, kind: MemTrapKind, tval: Word) -> XResult<T> {
        match (kind, err) {
            (MemTrapKind::Fetch, XError::AddrNotAligned) =>
                self.trap(TrapCause::Exception(Exception::InstructionMisaligned), tval),
            (MemTrapKind::Fetch, XError::BadAddress) =>
                self.trap(TrapCause::Exception(Exception::InstructionAccessFault), tval),

            (MemTrapKind::Load, XError::AddrNotAligned) =>
                self.trap(TrapCause::Exception(Exception::LoadMisaligned), tval),
            (MemTrapKind::Load, XError::BadAddress) =>
                self.trap(TrapCause::Exception(Exception::LoadAccessFault), tval),

            (MemTrapKind::StoreAmo, XError::AddrNotAligned) =>
                self.trap(TrapCause::Exception(Exception::StoreMisaligned), tval),
            (MemTrapKind::StoreAmo, XError::BadAddress) =>
                self.trap(TrapCause::Exception(Exception::StoreAccessFault), tval),

            (_, other) => Err(other),
        }
    }
}
```

重点是：

- 这个 helper 不是给每个调用点写 `match`
- 它只应该放在少数统一语义入口里用

## 真正需要做转换的地方，应该只有少数 façade

这是整个设计最重要的部分。

不要在每个 instruction handler 里各自转 trap。  
应该只在下面这些边界收口：

```rust
impl RVCore {
    fn fetch_inst(&mut self) -> XResult<u32>;
    fn decode_inst(&mut self, raw: u32) -> XResult<DecodedInst>;
    fn execute_inst(&mut self, raw: u32, inst: DecodedInst) -> XResult;

    fn mem_read_load(&mut self, vaddr: VirtAddr, size: usize) -> XResult<Word>;
    fn mem_write_store(&mut self, vaddr: VirtAddr, size: usize, value: Word) -> XResult;
    fn mem_read_amo(&mut self, vaddr: VirtAddr, size: usize) -> XResult<Word>;
    fn mem_write_amo(&mut self, vaddr: VirtAddr, size: usize, value: Word) -> XResult;
}
```

这些 façade 才负责：

- 把 `InvalidInst` 转成 `IllegalInstruction`
- 把 `BadAddress` / `AddrNotAligned` 转成对应的访存 trap

而 instruction handler 本身继续只写语义：

```rust
let value = self.mem_read_load(addr, size)?;
self.mem_write_store(addr, size, value)?;
```

这才是真正避免冗余代码的关键。

## `step()` 的统一处理方式

引入 `XError::Trap(PendingTrap)` 后，`step()` 可以非常自然地统一收口：

```rust
fn step(&mut self) -> XResult {
    if self.check_pending_interrupts() {
        self.retire();
        return Ok(());
    }

    let result = (|| {
        let raw = self.fetch_inst()?;
        let inst = self.decode_inst(raw)?;
        self.execute_inst(raw, inst)
    })();

    match result {
        Ok(()) => {
            self.retire();
            Ok(())
        }
        Err(XError::Trap(trap)) => {
            self.pending_trap = Some(trap);
            self.retire();
            Ok(())
        }
        Err(err) => Err(err),
    }
}
```

这个结构的优点是：

- `?` 仍然是主控制流
- trap 不再是 side effect + sentinel 的组合
- `retire()` / `commit_trap()` 完全不用推翻

## 对同步 trap 和异步 interrupt 的建议分工

这部分要提前说清楚，不然后面还会混。

### 同步 trap

例如：

- `IllegalInstruction`
- `Ecall`
- `Breakpoint`
- load/store/AMO fault
- 以后 MMU 的 page fault

我建议统一改成：

- **返回 `Err(XError::Trap(...))`**

### 异步 interrupt

例如当前 [`check_pending_interrupts()`](../../xemu/xcore/src/cpu/riscv/trap/handler.rs) 这种 pre-step interrupt sampling，我认为可以继续保留现在的 side effect 路线：

- `self.raise_trap(...)`
- 然后直接 `retire()`

因为它不是从深层调用栈向上传播出来的 fault，而是 step 边界主动采样的外部事件。

这意味着：

- `XError::Trap(...)` 主要服务于 **同步异常**
- `pending_trap` 继续作为 **统一提交点**
- `raise_trap()` 可以保留给 **interrupt/external injection**

这已经足够统一，而且不会过度重构。

## 这个设计如何解决 atomic 那个问题

当前 review finding 的本质是：

- atomic helper 里直接 `with_mem!(read/write)?`
- `AddrNotAligned` / `BadAddress` 被直接冒泡成 host error

在这套设计下，atomic 不需要到处补 `match`。

只要把 atomic 统一走 façade：

```rust
fn mem_read_amo(&mut self, vaddr: VirtAddr, size: usize) -> XResult<Word> {
    match with_mem!(read(self.virt_to_phys(vaddr), size)) {
        Ok(v) => Ok(v),
        Err(err) => self.trap_mem(err, MemTrapKind::StoreAmo, vaddr.as_usize() as Word),
    }
}

fn mem_write_amo(&mut self, vaddr: VirtAddr, size: usize, value: Word) -> XResult {
    match with_mem!(write(self.virt_to_phys(vaddr), size, value)) {
        Ok(()) => Ok(()),
        Err(err) => self.trap_mem(err, MemTrapKind::StoreAmo, vaddr.as_usize() as Word),
    }
}
```

那么 [`atomic.rs`](../../xemu/xcore/src/cpu/riscv/inst/atomic.rs) 里的代码仍然可以保持简洁：

```rust
let old = self.mem_read_amo(addr, 4)?;
self.mem_write_amo(addr, 4, new)?;
```

trap 会自动通过 `Err(XError::Trap(...))` 传回 `step()`，不需要 atomic helper 自己碰 `pending_trap`。

## 对现有代码的实际影响

### 需要改的地方

- [`xemu/xcore/src/error.rs`](../../xemu/xcore/src/error.rs)
  - 增加 `Trap(PendingTrap)` variant
- [`xemu/xcore/src/cpu/riscv/mod.rs`](../../xemu/xcore/src/cpu/riscv/mod.rs)
  - 增加 `trap` / `trap_mem` / `fetch_inst` / `decode_inst` / `execute_inst`
  - `step()` 统一消费 `Err(XError::Trap(...))`
- [`xemu/xcore/src/cpu/riscv/inst/base.rs`](../../xemu/xcore/src/cpu/riscv/inst/base.rs)
  - load/store 改成走 façade
- [`xemu/xcore/src/cpu/riscv/inst/atomic.rs`](../../xemu/xcore/src/cpu/riscv/inst/atomic.rs)
  - LR/SC/AMO 改成走 façade
- 之后再逐步把 `ecall` / `ebreak` / privilege violation 改成 `return self.trap(...)`

### 不需要大改风格的地方

这些地方仍然可以继续保持今天的简洁写法：

- RV64-only 宏继续 `return Err(XError::InvalidInst);`
- compressed 非法编码继续 `return Err(XError::InvalidInst);`
- `dispatch` 不匹配继续返回 `Err(XError::InvalidInst)`

因为最终是在 `decode_inst()` / `execute_inst()` 这类 façade 被统一翻成 `XError::Trap(...)`。

## 类型层面的一个实现细节

如果 `XError` 直接挂 `PendingTrap`，当前 [`error.rs`](../../xemu/xcore/src/error.rs) 的 derive 需要顺手调整。

现在 `XError` 是：

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
```

但 [`PendingTrap`](../../xemu/xcore/src/cpu/riscv/trap/cause.rs) / `TrapCause` 当前没有完整匹配这些 derive。

所以实现时有两个选择：

1. 给 `PendingTrap` / `TrapCause` / `Exception` / `Interrupt` 补齐所需 derive
2. 更简单一点，直接把 `XError` 的 derive 收到真正需要的最小集合，例如：

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
```

我倾向于第 2 种，因为 `XError` 通常并不需要 `Ord` / `Hash`。

## 关于架构耦合

这个方案唯一需要小心的一点是：

- [`error.rs`](../../xemu/xcore/src/error.rs) 目前是全局模块
- `PendingTrap` 是 RISC-V trap 类型

如果你担心把 `error.rs` 直接绑死到 RISC-V，可以这样处理：

```rust
#[cfg(riscv)]
pub type TrapPayload = crate::cpu::riscv::trap::PendingTrap;

pub enum XError {
    Trap(TrapPayload),
    ...
}
```

也就是说：

- `XError` 继续是统一错误类型
- 但 trap payload 通过 arch-local alias 注入

这已经足够，不需要为了“通用性”额外造更重的抽象。

## 推荐的落地顺序

### Phase A. 先把 `InvalidInst` 收口

先只改：

- `decode_inst(raw)`：`InvalidInst -> XError::Trap(IllegalInstruction, raw)`
- `execute_inst(raw, inst)`：`InvalidInst -> XError::Trap(IllegalInstruction, raw)`
- `step()`：统一消费 `Err(XError::Trap(...))`

这是最小、最安全的一步。

### Phase B. 再把访存 fault 收口到 façade

引入：

- `mem_read_load`
- `mem_write_store`
- `mem_read_amo`
- `mem_write_amo`

把 `BadAddress` / `AddrNotAligned` 在这些边界统一翻成 `XError::Trap(...)`。

这一步完成后，atomic 那个 review finding 会自然消失。

### Phase C. 最后逐步替换同步 `raise_trap()`

再把：

- `ecall`
- `ebreak`
- privilege violation
- CSR illegal access

逐步改成：

```rust
return self.trap(...);
```

这样同步异常路径就会完全统一。

## 最终建议

如果只保留一句话，我建议这样定规则：

**所有同步 guest trap 都编码成 `Err(XError::Trap { cause, tval })` 或 `Err(XError::Trap(PendingTrap))`，只在 `RVCore::step()` 统一搬运到 `pending_trap` 并提交；其他 `XError` 继续保留为真正的 host/internal error。**

这是目前在“少改代码、保持简洁、兼容后续 MMU/Bus”这几个目标之间，最平衡的一种方案。
