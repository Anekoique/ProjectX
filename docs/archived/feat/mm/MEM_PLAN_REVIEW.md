# MEM_PLAN 设计复评

> 针对 [MEM_PLAN.md](./MEM_PLAN.md) 的第二轮实现前评审
> 评审时间：2026-03-23
> 重点：在不把设计做重的前提下，把 Bus / MMU / TLB / MMIO 的边界收紧

## 结论

这一版 `MEM_PLAN` 比前面的版本明显更好。

最大的进步不是“类型更多”或“设计更全”，而是开始主动减少多余的 enum / struct，把接口压到能工作的最小集合。这条方向是对的，应该继续坚持。

结合当前代码和本地参考实现：

- `KXemu` 证明了 `CPU { cores, shared bus }` 这条 ownership 路线完全成立
- `REMU` / `Nemu-rust` 证明了 **Bus 内部分离 RAM 和 MMIO** 比“所有 region 都塞进同一种 trait object”更干净
- 你当前 `MEM_PLAN` 的公开接口已经足够简洁，不建议为了“架构完整感”再额外堆很多中间类型

我的总体判断是：

- 这份 plan **不需要推翻**
- 但还需要补 5 个关键设计点，才能真正做到 clean, concise, elegant

## 当前方案最值得保留的部分

下面这些设计我建议明确保留，不要因为追求“更抽象”反而做重：

- `CPU` 持有 `bus`，并在执行时把 `&mut Bus` 显式传给 hart
- `RVCore` 内持有 `Mmu`，TLB 也是 hart-local
- `Device` 保持极小接口：`read(offset, size)` / `write(offset, size, value)`
- `Bus` 只处理 physical dispatch，不懂虚拟地址和 privilege
- `MMU` 只负责翻译和权限检查，不直接做 trap side effect
- `sfence.vma` 和 `satp` side effect 最终都落到同一套 TLB flush 机制

这套方向和当前项目的体量是匹配的。不要为了模仿成熟模拟器，把自己带到“接口很多，但每层只薄薄包一层”的坏味道里。

## 从参考实现里真正值得借的东西

### 1. `CPU { cores, bus }` 是对的，但它本质上已经是 machine/container

`KXemu` 的 `RVCPU` 就是这个结构：

- `RVCPU::init(Bus *bus, ..., coreCount)` 创建多个 core
- 所有 core 共享同一份 `bus`
- `RVCPU::step()` / `run()` 负责调度各个 core

这个模式说明：

- `bus` 放进 `CPU` 没问题
- 真正重要的是 **shared bus 只能有一份**
- `CPU` 此时已经不只是“单核包装器”，而是在扮演 machine/container

所以这一点我建议在 plan 里直接写明，不要再纠结命名：

```rust
pub struct CPU {
    cores: Vec<RVCore>,
    bus: Bus,
    state: State,
    ...
}
```

### 2. Bus 内部最好分成 RAM 和 MMIO 两个子域

这点是这轮 review 最重要的新增意见。

`KXemu`、`REMU`、`Nemu-rust` 虽然接口风格不同，但有一个共同点：

- **RAM 和 MMIO 在 Bus 内部是分开的**

这不是“实现偏好”，而是因为下面三类访问天然只该落到 RAM：

- page table walk
- ELF / image load
- 未来真正的原子读改写路径

如果继续坚持：

```rust
pub struct Bus {
    regions: Vec<Region>, // 所有东西都是 Box<dyn Device>
}
```

那么很快就会遇到几个脏点：

- page walker 需要先判断“这个 region 能不能当页表页读”
- image loader 需要考虑“是否要 downcast 到 Ram 才能 bulk load”
- 以后 atomics/CAS 也会自然要求 memory-only path

相比之下，更简洁的做法反而是**内部显式分层**：

```rust
pub struct Bus {
    memory: Memory,
    mmio: Vec<MmioRegion>,
}

pub struct MmioRegion {
    name: &'static str,
    base: usize,
    size: usize,
    dev: Box<dyn Device>,
}
```

这里的关键点是：

- **公开接口仍然可以保持很小**
- 只是 Bus 内部不要把 RAM 和 MMIO 混成一锅

这比引入 `RegionKind + downcast + 特判` 更干净，也更符合“少类型但边界清楚”的目标。

### 3. `REMU` 值得借的不是“方法多”，而是 fault 分层

`REMU` 很值得借鉴的一点是：

- `Bus` / `Memory` 层返回的是 **ISA-agnostic error**
- trap mapping 留在更上层

这和你 CSR 子系统已经形成的分层思路是一致的。

所以我建议这里保持同样风格：

- `Bus` 返回 `BusError`
- `MMU` 返回 `MemFault`
- `RVCore` 再把这些 fault 映射成 RISC-V trap cause

不要让 `MMU` 或 `Bus` 直接操作 `pending_trap`。

## 我建议修正的 5 个关键设计点

### 1. 不要同时保留 `Perm`、`AccessType`、`PageFault` 这三层近义抽象

当前 plan 里最容易继续膨胀的地方，就是“为了表达精确”而引入多个相互投影的类型。

如果目标是简洁，我建议把访问语义压缩成一套上下文：

```rust
pub enum AccessKind {
    Fetch,
    Load,
    Store,
    Amo,
    PageTableWalk,
}

pub struct AccessCtx {
    pub kind: AccessKind,
    pub eff_priv: PrivilegeMode,
    pub sum: bool,
    pub mxr: bool,
    pub satp: Word,
}

pub enum MemFault {
    Page,
    Access,
}
```

这样有几个好处：

- 不需要再单独维护 `Perm`
- 不需要再公开单独的 `PageFault` struct
- `AccessKind` 本身就足以推导“要检查 R/W/X 哪一类权限”
- misaligned 不必进入 `MemFault`，因为它本来就应该在 `RVCore` 进入 MMU 前处理

我的建议是：

- `AccessKind` 负责“这次访问是什么”
- `RVCore` 负责“这次访问失败时应该 raise 哪个 trap”
- `MMU` 只返回“这是 page fault 还是 access fault”

这比 `Perm + AccessType + PageFault` 更紧凑。

### 2. page walk 必须走 memory-only path，而不是 generic bus read

这一点需要从“原则建议”提升为“接口约束”。

页表遍历不是普通 guest load，它有两个本质区别：

- 访问的是 **physical memory**
- 不允许因为 guest 配错 `satp`，就误读某个 MMIO 设备寄存器

所以我建议不要让 `page_walk()` 走：

```rust
bus.read(paddr, size)
```

而是走 Bus 的 memory-only 内部入口，例如：

```rust
bus.read_mem(paddr, size)
```

这个 `read_mem` 不一定要暴露成外部公共 API，但在 Bus 内部应明确存在。

这样可以保证：

- 命中 RAM：继续读 PTE
- 命中 MMIO：直接失败
- 未命中：直接失败

这个约束同样适用于：

- `CPU::load()` / ELF/image load
- 以后如果要做真正的 memory-backed atomic path

### 3. 当前代码迁移时，`MemOps` 应该整体消失，不要保留半层旧抽象

结合现在的代码结构：

- [`xemu/xcore/src/cpu/mod.rs`](../../xemu/xcore/src/cpu/mod.rs) 里 `CPU` 还持有单个 `core`
- [`xemu/xcore/src/cpu/core.rs`](../../xemu/xcore/src/cpu/core.rs) 里 `CoreOps::step()` 还不接 `Bus`
- [`xemu/xcore/src/cpu/mem.rs`](../../xemu/xcore/src/cpu/mem.rs) 里还有 `MemOps`
- [`xemu/xcore/src/memory/mod.rs`](../../xemu/xcore/src/memory/mod.rs) 还提供全局 `MEMORY + with_mem!`
- [`xemu/xcore/src/cpu/riscv/inst/base.rs`](../../xemu/xcore/src/cpu/riscv/inst/base.rs) 和 [`xemu/xcore/src/cpu/riscv/inst/atomic.rs`](../../xemu/xcore/src/cpu/riscv/inst/atomic.rs) 直接打到 `with_mem!`

到了 Phase 3，我建议直接做成下面这张迁移表，而不是保留旧层：

| 当前 | Phase 3 后 |
|---|---|
| `static MEMORY` + `with_mem!` | `CPU.bus` |
| `MemOps::virt_to_phys()` | `RVCore.mmu.translate()` |
| `MemOps::init_memory()` | `CPU::load()` / `Bus::load_*()` |
| `CoreOps::step(&mut self)` | `CoreOps::step(&mut self, bus: &mut Bus)` |
| `Memory::fetch_u32/read/write/load` | `Memory + Bus` 内部分层 |

重点是：

- **不要让 `MemOps` 在 MMU 引入后继续存活**
- 否则最后会出现“旧 identity mapping 抽象”和“新 MMU 抽象”并存的半重构状态

### 4. 对齐规则必须上移到 `RVCore`，RAM 保持 raw bytes

当前 [`memory/mod.rs`](../../xemu/xcore/src/memory/mod.rs) 里已经把两种语义混在一起了：

- 物理内存字节读写
- 架构层的 alignment policy

尤其是：

- 普通 `read/write` 要求自然对齐
- `fetch_u32` 又允许 2-byte 对齐

这在单个 `Memory` 里还能工作，但有了 MMU / Bus 后会开始变脏。

我的建议是把边界彻底收清：

- `Memory` / `Ram`：只做 little-endian raw byte access
- `RVCore::fetch()`：负责 IALIGN=16 的 2-byte 对齐规则
- `RVCore::load/store/amo`：负责自然对齐检查
- `Bus`：只做 physical dispatch 和 region width validation

换句话说：

- **RAM 不应该再返回 `AddrNotAligned`**
- misaligned trap 的责任必须回到 core

这点和你前面对 atomic review 里暴露出来的问题其实是同一类边界问题。

### 5. A extension 已经存在，Bus 设计不能把以后真正的原子路径堵死

这次 review 的一个代码层现实是：

- A extension 已经实现了一版
- 当前 `LR/SC/AMO` 还直接依赖全局 memory

所以 Bus 设计时需要提前避免一个未来坑：

- 不要把 RAM 完全包成“只能 generic read/write 的 opaque device”

原因很简单：

- `LR/SC` 的 reservation 仍应留在 `RVCore`
- 但真正的 memory-backed AMO/CAS 能力以后很可能需要 Bus/Memory 提供更直接的原子入口
- MMIO 通常又不应支持这一套

因此我建议：

- **Phase 3 不必现在就引入 `Bus::amo_*` API**
- 但 Bus 的内部结构必须允许未来为 RAM 单独加这类能力

这再次说明：

- 把 RAM 和 MMIO 在 Bus 内部拆开，是更稳的设计
- 继续把所有 region 都当 `Box<dyn Device>`，后面只会让 A extension 的实现越来越拧

## 一个更贴近当前项目的最小架构

我建议最终朝下面这版结构收敛：

```rust
pub struct CPU {
    cores: Vec<RVCore>,
    bus: Bus,
    state: State,
    ...
}

pub struct Bus {
    memory: Memory,
    mmio: Vec<MmioRegion>,
}

pub struct RVCore {
    mmu: Mmu,
    csr: CsrFile,
    privilege: PrivilegeMode,
    reservation: Option<Reservation>,
    ...
}

pub struct Mmu {
    tlb: Tlb,
}
```

这里真正需要长期稳定的接口，建议只有下面几个：

```rust
impl CPU {
    pub fn step_hart(&mut self, hartid: usize) -> XResult;
}

impl RVCore {
    pub fn step(&mut self, bus: &mut Bus) -> XResult;
}

impl Mmu {
    pub fn translate(
        &mut self,
        vaddr: VirtAddr,
        ctx: AccessCtx,
        bus: &mut Bus,
    ) -> Result<PhysAddr, MemFault>;
}

impl Bus {
    pub fn read(&mut self, paddr: usize, size: usize) -> Result<Word, BusError>;
    pub fn write(&mut self, paddr: usize, size: usize, value: Word) -> Result<(), BusError>;
}
```

请注意这里有两个刻意的“少”：

- 少公开类型
- 少公开方法

但内部边界反而更清楚：

- `CPU` 只负责调度和持有 shared bus
- `RVCore` 只负责架构执行与 trap mapping
- `MMU` 只负责翻译
- `Bus` 只负责 physical memory/MMIO dispatch

## 对 TLB / `satp` / `sfence.vma` 的补充建议

这部分我建议在 plan 里说得更明确一点：

- TLB 是 **hart-local**
- `satp` write side effect 只 flush 当前 hart 的 TLB
- `sfence.vma` 也是当前 hart 的指令 side effect
- 将来如果支持 remote fence / IPI，那是更高层的多核协调问题，不应耦合进 Bus

这和 `KXemu` 的做法是一致的，也更符合你当前 `mmu` 在 `RVCore` 内部的结构。

## 推荐的实现顺序

建议把顺序再收敛成下面这版：

### Step 0. 先改 ownership 和执行接口

- `CPU` 改成 `cores + bus`
- `CoreOps::step()` 改成显式接收 `&mut Bus`
- 删除 `MemOps` 的存在前提

### Step 1. 先把今天的 `Memory` 拆成 `Memory + Bus`

- 先保持 Bare 模式
- 不做 MMU
- 目标只是移除 `static MEMORY` 和 `with_mem!`

### Step 2. 改 `fetch/load/store/amo` 入口

- [`base.rs`](../../xemu/xcore/src/cpu/riscv/inst/base.rs) 通过 Bus 访存
- [`atomic.rs`](../../xemu/xcore/src/cpu/riscv/inst/atomic.rs) 通过 Bus 访存
- 对齐规则同时从 `Memory` 上移到 `RVCore`

### Step 3. 引入 `AccessCtx + MemFault`，再接 MMU skeleton

- 先把接口定型
- 裸跑模式继续 identity map
- 不要先急着写完整 page walk

### Step 4. 做 SV32 / SV39 page walk

- canonical address
- superpage alignment
- `SUM` / `MXR`
- `MPRV`
- PTE walk 走 memory-only path

### Step 5. 做 TLB 和 flush side effects

- `satp` write side effect
- `sfence.vma`
- hart-local flush

### Step 6. 最后再接 MMIO routing 和 Phase 4 设备

- 这时 Bus 的边界已经稳定
- 设备加进来不会反向污染 MMU 和 page walk 设计

## 最终判断

如果只保留一句最核心的 review 结论，我会这样写：

**这版 `MEM_PLAN` 正确的方向不是“继续抽象”，而是“在保持接口最小化的同时，把 RAM、MMIO、MMU、trap 的边界切干净”。**

如果只允许我提出 4 条最重要的修改建议，我会选：

1. **保留 `CPU { cores, bus }`，但明确它是 shared-machine container**
2. **Bus 内部显式分开 RAM 和 MMIO，不要把所有 region 都塞成同一种 `Device`**
3. **用 `AccessCtx + MemFault` 收掉 `Perm / AccessType / PageFault` 这类重复抽象**
4. **让 `MemOps` 直接退出历史舞台，避免新旧访存模型并存**

把这 4 点补上以后，我认为这份 plan 就不只是“方向对”，而是已经接近“可以直接开工，而且不容易返工”的状态了。
