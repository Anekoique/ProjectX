# MEM 实现复审（最近 5 个内存子系统提交）

> 范围：`54e7f9c..4d5ad3c`
> 对照文档：[MEM_PLAN.md](./MEM_PLAN.md)
> 复审时间：2026-03-26
> 基线：在干净 worktree 中检视 `4d5ad3c` 的提交态代码，不混入当前未提交修改
> 参考：RISC-V Privileged ISA Manual（官方仓库：<https://github.com/riscv/riscv-isa-manual>）

## Update Log

- `2026-03-26 / v1`
  - 初版复审，范围为 `dd11797..4d5ad3c`
  - 重点覆盖 MMU / PMP / TLB / `sfence.vma` / CSR side effect
  - 结论为 7 个高置信问题：`5 HIGH + 2 MEDIUM`
- `2026-03-26 / v2`
  - 按要求扩展范围到 `54e7f9c..4d5ad3c`
  - 补审 `Bus` / `Ram` / `Device` trait / CPU 与 Bus 的接线
  - 新增 2 个 device-layer 相关 `MEDIUM`
  - 当前结论更新为 9 个高置信问题：`5 HIGH + 4 MEDIUM`
- `2026-03-26 / v3`
  - 读取 `MEM_IMPL_FIX_REPORT.md` 并复审当前修改后的代码
  - 重新按官方 RISC-V manual 校对 MMU / PMP / CSR / device 行为
  - 运行验证：
    - `make test ARCH=riscv64`
    - `make test ARCH=riscv32`
    - `make clippy ARCH=riscv64`
    - `make clippy ARCH=riscv32`
  - 结论：前一轮多数问题已修复，但当前树上仍残留 `4 HIGH`
- `2026-03-26 / v4`
  - 继续按 manual 复查 fetch 路径
  - 新确认 1 个 instruction-fetch 跨页残留问题
  - 当前 fix pass 结论更新为：`5 HIGH`
- `2026-03-26 / v5`
  - 重新逐项对照当前工作树代码和 `MEM_IMPL_FIX_REPORT.md`
  - 修正 v3/v4 中 3 个已经在后续代码里修掉的误报：
    - RV64 odd `pmpcfg` illegal CSR
    - locked PMP write 的 CSR readback 回写
    - final PMP size-aware check
  - 当前 fix pass 的最新结论更新为：`3 HIGH + 1 MEDIUM`
- `2026-03-26 / v6`
  - 再次复审最新 fix 后的当前工作树
  - 确认 v5 中的 fetch 跨页问题和 RV64 PTE PPN 宽度问题都已修复
  - 新确认 1 个此前遗漏的 PMP priority/matching 语义问题
  - 文档侧仍有 1 个 stale 描述问题，另外 `make clippy ARCH=riscv32` 仍有 2 个 warning
  - 当前最新结论更新为：`1 HIGH + 2 MEDIUM`

## 当前状态（v6 复核，以本节为准）

重新对照 `MEM_IMPL_FIX_REPORT.md`、当前工作树代码和官方 RISC-V privileged manual 后，可以确认这轮 fix 已经进一步修掉了 v5 时还残留的几个主要问题，尤其是：

- `SUM` 不再错误放宽 S-mode fetch
- `MPRV` final PMP privilege 已修正
- RV64 odd `pmpcfg` CSR 现在会直接报 illegal instruction
- locked PMP write-ignore 的 CSR readback 已回写成实际 PMP state
- final PMP check 已经按 access size 接通到 `load/store/AMO`
- LR/SC 的 load/store fault class 已修正
- reset 会清空 `mmu` / `pmp`
- `Bus` / `Ram` 的大部分越界处理已经从 panic 改成 `Err`
- fetch 已改成 2-byte parcel 方式，跨页时会对 `pc + 2` 再做独立 translate
- RV64 的 Sv39/Sv48/Sv57 PTE 现在已经显式使用 44-bit PPN 宽度

本轮没有再发现新的 device-layer 高置信残留问题。

但 correctness 仍然不能算已经被完全 ensured。当前工作树上还剩 `1 HIGH + 2 MEDIUM`。

### [HIGH] PMP 仍然没有实现 “lowest-numbered entry that matches any byte wins” 的完整语义，partial-overlap 访问还可能被错误放行

**位置**

- `xemu/xcore/src/cpu/riscv/pmp.rs:53-71`
- `xemu/xcore/src/cpu/riscv/pmp.rs:130-149`

**问题**

manual 对 PMP 的优先级规则是：

- lowest-numbered PMP entry that matches **any byte** of an access 决定结果
- 这个 entry 必须覆盖访问的 **all bytes**
- 否则访问必须失败，不能继续让更低优先级 entry 再参与判定

当前实现的 `Pmp::check()` 只把 “完整覆盖整个 `[paddr, paddr + size)`” 当作 matched：

```rust
let matched = entry.contains(paddr, size, prev_addr);
if !matched {
    continue;
}
```

这会漏掉一个关键情形：如果较高优先级 entry 只覆盖了这次访问的**一部分字节**，当前实现不会把它视为 match-fail，而是会继续往后看更低优先级 entry。

这和 manual 不一致。partial overlap 本来应该直接 fail。

一个具体反例是：

- `entry0` 覆盖访问的一部分字节
- `entry1` 覆盖整个访问并且权限允许

当前代码会错误地由 `entry1` 放行，而 spec 要求由 `entry0` 先命中并导致失败。

**影响**

PMP priority/matching 语义仍然偏离标准，在有重叠 region 或跨 region 访问时，可能错误放行本应失败的访问。

**修正建议**

在 `Pmp::check()` 中，不仅要区分 “full match”，还要区分 “matches any byte but not all bytes”：

- full match: 按当前 entry 的权限决定 success/fail
- partial overlap: 立即 fail
- no overlap: 才继续看下一个 entry

### [MEDIUM] `MEM_IMPL_FIX_REPORT.md` 的当前表述已经比代码更“乐观”，会误导后续读者对 correctness 的判断

**位置**

- `docs/mem/MEM_IMPL_FIX_REPORT.md:57-63`
- `docs/mem/MEM_IMPL_FIX_REPORT.md:88-90`
- `docs/mem/MEM_IMPL_FIX_REPORT.md:107-109`

**问题**

这个 fix report 已经比前一版好很多，但现在还残留几处 stale 内容：

- Fix 2 里的示例代码仍然写着 `self.pmp.check(paddr, 1, op, priv_mode)`
- Fix 5 里的示例代码仍然使用旧签名 `self.translate(addr, MemOp::Load)`
- Fix 7 的描述把“full access range”说成已经完全修好，但没有提到上面的 PMP partial-overlap residual

**影响**

后续读者如果只看 fix report，会误以为相关实现和代码完全一致。

**修正建议**

把示例代码和剩余 issue 描述同步到当前实际实现。

### [MEDIUM] `make clippy ARCH=riscv32` 仍然有 2 个 warning，说明验证尚未 clean

**位置**

- `xemu/xcore/src/cpu/riscv/mem.rs:98`
- `xemu/xcore/src/cpu/riscv/mem.rs:105`

**问题**

新的 fetch 实现里有两个对 `u32` 的同类型强转：

- `let lo = ... as u32`
- `let hi = ... as u32`

在 RV32 配置下这会触发 `clippy::unnecessary_cast` warning。

**影响**

不影响功能正确性，但按当前仓库的验证标准，这一轮 fix 还不能算 completely clean。

**修正建议**

去掉这两个冗余强转，或者用一个不会在 RV32 下触发 warning 的统一写法。

## 当前验证结果

当前修改树在本地验证中：

- `make test ARCH=riscv64`：通过
- `make test ARCH=riscv32`：通过
- `make clippy ARCH=riscv64`：通过
- `make clippy ARCH=riscv32`：通过，但有 2 个 warning

这说明剩余问题主要是：

- 1 个测试未覆盖到的 spec 级残留问题
- 1 个文档同步问题
- 1 个 clippy cleanliness 问题

## 结论

这 5 个提交已经把 `MEM_PLAN` 的主干骨架落下来了：

- `54e7f9c` 已经把全局 `MEMORY` 替换成 `Bus` device layer，并接到 `Arc<Mutex<Bus>>`
- `Bus` / `MMU` / `PMP` / `TLB` 的层次基本建立
- `satp` / `mstatus` side effect、`sfence.vma`、SV32/SV39 page walk 都已经接通
- 单测在 RV32 / RV64 下都能跑通

但从当前 fix pass 后的 correctness 角度看，这一版还不能算 fully correct。

原始 9 个高置信问题里，大部分已经修掉；按当前工作树最新复核后，还剩：

- 1 个 `HIGH`
- 2 个 `MEDIUM`

其中唯一剩下的 `HIGH` 仍然是手册语义层面的行为问题，不是风格问题。

## 复审方法

- 以提交态代码为准，不看当前 dirty worktree
- 重点检查：
  - `xemu/xcore/src/device/bus.rs`
  - `xemu/xcore/src/device/ram.rs`
  - `xemu/xcore/src/device/mod.rs`
  - `xemu/xcore/src/cpu/core.rs`
  - `xemu/xcore/src/cpu/riscv/mmu.rs`
  - `xemu/xcore/src/cpu/riscv/pmp.rs`
  - `xemu/xcore/src/cpu/riscv/mem.rs`
  - `xemu/xcore/src/cpu/riscv/csr/mod.rs`
  - `xemu/xcore/src/cpu/riscv/csr/ops.rs`
  - `xemu/xcore/src/cpu/riscv/inst/privileged.rs`
  - `xemu/xcore/src/cpu/riscv/inst/atomic.rs`
- 对照 `MEM_PLAN.md` 和官方 privileged spec 校对：
  - `SUM` / `MXR`
  - `MPRV`
  - `sfence.vma`
  - PMP CSR layout / locking / matching semantics
  - LR/SC fault class

> 注：下面 `Findings` 小节是 v1/v2 的历史复审记录，保留用于追踪问题演进；当前 fix pass 后的残留问题以上面的“当前状态（v6 复核，以本节为准）”为准。

## Findings

### [HIGH] S-mode 在 `SUM=1` 时错误地允许从 `U=1` 页面取指

**位置**

- `xemu/xcore/src/cpu/riscv/mmu.rs:144-162 @ 4d5ad3c`
- `xemu/xcore/src/cpu/riscv/mmu.rs:299-314 @ 4d5ad3c`

**问题**

当前实现把 `SUM` 当成了 “S-mode 可访问 U-page” 的通用开关。

这对 load/store 是对的，但对 instruction fetch 不对。根据 privileged spec，**无论 `SUM` 是否置位，Supervisor 都不能执行 `U=1` 页上的代码**。

也就是说，下面这段逻辑：

- TLB hit path 的 `permits()`
- page walk leaf 权限检查的 `check_perm()`

都把 `Fetch` 和 data access 混在了一起处理，导致：

- `S-mode`
- `SUM=1`
- `PTE.U=1`
- `PTE.X=1`

会被错误放行。

**影响**

Supervisor 取指权限被放宽，和 spec 不一致。

**修正建议**

把 `MemOp::Fetch` 单独处理：

- `priv_mode == User` 时，`U=1` 页可执行
- `priv_mode == Supervisor` 时，`U=1` 页必须 fault
- `SUM` 只影响 data access，不影响 fetch

### [HIGH] 最终 PMP 检查没有使用 effective privilege，`MPRV` 数据访问会被错误放行

**位置**

- `xemu/xcore/src/cpu/riscv/mem.rs:15-22 @ 4d5ad3c`
- `xemu/xcore/src/cpu/riscv/mem.rs:25-38 @ 4d5ad3c`

**问题**

当前 `translate()` 的流程是：

1. data access 时先用 `effective_priv()`
2. 把这个 privilege 传给 `MMU::translate()`
3. 之后再做一次最终 `PMP::check()`

但第 3 步传进去的是 `self.privilege`，不是 effective privilege：

```rust
self.pmp.check(paddr, op, self.privilege)
```

这会导致：

- 当前 hart 处于 `M-mode`
- `mstatus.MPRV=1`
- `mstatus.MPP=S/U`

时，地址翻译按 S/U 做了，但最终 PMP 仍按 M-mode 做。这样本来应该受限的数据访问会被 M-mode bypass 掉。

**影响**

`MPRV` 语义不完整，PMP 对 data access 的限制会失效。

**修正建议**

最终 `PMP::check()` 对 data access 必须使用和 MMU 相同的 effective privilege；只有 instruction fetch 继续使用 `self.privilege`。

### [HIGH] RV64 的 PMP CSR 布局接错了，`pmpcfg2` 无法正确配置 entry 8..15

**位置**

- `xemu/xcore/src/cpu/riscv/csr/mod.rs:129-149 @ 4d5ad3c`
- `xemu/xcore/src/cpu/riscv/csr/ops.rs:39-49 @ 4d5ad3c`

**问题**

当前实现把 `pmpcfg0..pmpcfg3` 全部注册成可访问 CSR，并且用：

```rust
let base = (addr - 0x3A0) as usize * std::mem::size_of::<Word>();
```

来映射 entry 下标。

这在 RV32 下勉强成立，但在 RV64 下不对：

- RV64 只有偶数号 `pmpcfg` CSR 是合法的
- `pmpcfg1` / `pmpcfg3` 在 RV64 应该是 illegal
- RV64 的 `pmpcfg0` 管 `entry 0..7`
- RV64 的 `pmpcfg2` 管 `entry 8..15`

当前映射会产生两个问题：

- `pmpcfg1` / `pmpcfg3` 被错误接受
- `pmpcfg2` / `pmpcfg3` 会映射到 `entry 16+`，实际更新被 `index < PMP_COUNT` 静默丢掉

结果就是：**spec-compliant 软件在 RV64 上写 `pmpcfg2` 时，entry 8..15 根本配不上去。**

**影响**

RV64 PMP CSR 行为和标准不兼容。

**修正建议**

- CSR 表按 `isa32` / `isa64` 分开注册
- RV64 只暴露合法的偶数号 `pmpcfg`
- side effect 显式把 `pmpcfg0 -> 0..7`、`pmpcfg2 -> 8..15`

### [HIGH] Locked PMP entry 仍然可以被软件改写

**位置**

- `xemu/xcore/src/cpu/riscv/pmp.rs:65-75 @ 4d5ad3c`

**问题**

`update_cfg()` 和 `update_addr()` 只是按下标直接覆盖：

- 没有检查 entry 自己是否 `L=1`
- 也没有处理 TOR 的前一项地址寄存器被锁住的规则

根据 spec：

- 若 `pmp[i].L=1`，写 `pmpcfg[i]` 和 `pmpaddr[i]` 应被忽略
- 若 `pmp[i].L=1` 且 `pmp[i].A=TOR`，写 `pmpaddr[i-1]` 也应被忽略

当前实现只在 access check 时看 `locked()`，配置路径完全没锁。

**影响**

已经 lock 的 PMP 区域可以被后续软件重新编程，破坏 PMP 的基本语义。

**修正建议**

在 `update_cfg()` / `update_addr()` 中实现真正的 lock 语义，而不是只在 `check()` 里读取 `L` 位。

### [HIGH] `lr.w` / `lr.d` 预翻译时按 `Amo` 做权限检查，可能报成 store fault

**位置**

- `xemu/xcore/src/cpu/riscv/inst/atomic.rs:53-59 @ 4d5ad3c`
- `xemu/xcore/src/cpu/riscv/inst/atomic.rs:71-84 @ 4d5ad3c`
- `xemu/xcore/src/cpu/riscv/mem.rs:41-60 @ 4d5ad3c`

**问题**

`lr.*` 现在先做一遍：

```rust
let paddr = self.translate(addr, MemOp::Amo)?;
```

然后再调用正常的 `load()`。

问题在于 `MemOp::Amo` 在 trap 映射里会落到 store/AMO fault 类别，而 `LR` 本质上是 load-like 指令。

因此如果某地址：

- 读允许
- 写不允许

那么 `lr.*` 可能在预翻译阶段就因为 “没有写权限” 提前失败，并且 fault class 也会更接近 store/AMO，而不是 load。

**影响**

LR/SC 语义和 trap 分类不干净，边界场景会和 spec 不一致。

**修正建议**

`LR` 获取 reservation 时应按 `Load` 路径完成翻译与权限检查，再记录物理地址 reservation。不要先用 `MemOp::Amo` 预翻译一遍。

### [MEDIUM] `reset()` 没有清空 MMU / PMP 状态

**位置**

- `xemu/xcore/src/cpu/riscv/mod.rs:102-111 @ 4d5ad3c`

**问题**

`RVCore::reset()` 目前会重置：

- GPR
- PC / NPC
- CSR
- privilege
- pending trap
- reservation

但不会重置：

- `self.mmu`
- `self.pmp`

这意味着 reset 之后下面这些状态会残留：

- TLB entries
- cached `satp` / `asid`
- cached `SUM` / `MXR`
- PMP entries
- PMP lock state

而 CSR 已经被清零了，二者会失去一致性。

**影响**

reset 后 hart 的实际内存保护状态和 CSR 可见状态不一致。

**修正建议**

在 `reset()` 中重新初始化 `self.mmu = Mmu::new()` 和 `self.pmp = Pmp::new()`。

### [MEDIUM] PMP 只按起始地址匹配，没有检查整个访问区间是否完全落在同一 entry 内

**位置**

- `xemu/xcore/src/cpu/riscv/pmp.rs:77-126 @ 4d5ad3c`

**问题**

`Pmp::check()` 只接收：

```rust
check(&self, paddr: usize, op: MemOp, priv_mode: PrivilegeMode)
```

也就是只有起始地址，没有 access size。

但 spec 要求：

- 优先级最高的匹配 entry 决定本次访问
- 该 entry 必须覆盖此次访问的所有字节
- 如果只覆盖了一部分，本次访问仍然应失败

当前实现无法表达这个条件，因此会错误接受跨边界访问。

**影响**

宽访问或边界访问时，PMP 行为会偏松。

**修正建议**

把 `size` 也传进 `Pmp::check()`，按 `[paddr, paddr + size)` 做完整覆盖判断。

### [MEDIUM] `Bus` 的区间判断使用了未检查加法，泛型 API 输入可能导致 overflow 后误判甚至 panic

**位置**

- `xemu/xcore/src/device/bus.rs:85-99 @ 4d5ad3c`

**问题**

`Bus` 当前的关键区间判断是：

```rust
if off + size <= self.ram.len() { ... }
if addr >= r.base && addr + size <= r.base + r.size { ... }
```

这里都没有做 `checked_add()`。

在当前 CPU 主路径里，`size` 基本只有 `1/2/4/8`，所以暂时不容易撞到。但 `Bus` 自己暴露的是通用接口：

- `read(addr, size)`
- `write(addr, size, value)`
- `read_ram(addr, size)`
- `load_ram(addr, data)`

只要未来有更泛化的调用，或者某处把异常 `size` 传进来，`off + size` / `addr + size` 就可能 wrap，随后：

- 把非法访问误判成合法
- 最终在 `Ram` 切片或 MMIO 路由里 panic

**影响**

device layer 的错误处理在边界输入下不是 fail-safe 的，可能从 `Err(BadAddress)` 退化成 host panic。

**修正建议**

把所有区间判断统一改成 `checked_add()` 形式，不要依赖小 size 的隐含前提。

### [MEDIUM] `Ram` 的公开 API 返回 `XResult`，但越界或非法 size 时会直接 panic，而不是返回错误

**位置**

- `xemu/xcore/src/device/ram.rs:18-34 @ 4d5ad3c`

**问题**

`Ram::read()` / `write()` / `load()` 都直接做切片：

```rust
buf[..size].copy_from_slice(&self.data[offset..offset + size]);
self.data[offset..offset + size].copy_from_slice(...);
self.data[offset..offset + data.len()].copy_from_slice(data);
```

这意味着只要：

- `offset + size` 越界
- `size > size_of::<Word>()`

就会直接 panic。

但这三个函数的签名都是 `XResult`，从接口语义上看，调用者会自然认为这是“可恢复错误”，而不是 “可能直接把 emulator 打崩”。

当前正常路径大多由 `Bus` 先做了边界筛选，所以问题暂时被遮住了；但这并不改变 `Ram` API 自己的行为和返回类型不一致。

**影响**

device 层的底座 API 不够稳，后续只要有绕过 `Bus` 的调用、测试辅助、或新增批量访问路径，就可能出现非预期 panic。

**修正建议**

给 `Ram` 自己补齐边界检查和 size 检查，越界时返回 `Err(XError::BadAddress)`，而不是依赖上层“保证不会传错”。

## 测试结论

我在独立干净 worktree 中对提交态 `4d5ad3c` 运行了：

```bash
make test ARCH=riscv64
make test ARCH=riscv32
```

结果：

- RV64: pass
- RV32: pass

这说明当前测试集已经覆盖了大部分主路径，但**还没有覆盖这次复审指出的 spec 边界**，尤其是：

- S-mode fetch `U=1` page + `SUM=1`
- `MPRV` + PMP final check
- RV64 `pmpcfg2`
- locked PMP write-ignore semantics
- LR fault class
- reset 后 MMU/PMP 清空
- PMP 多字节覆盖语义
- `Bus` / `Ram` 的 overflow-safe / non-panicking 边界行为

## 建议的补测项

建议至少补下面这些测试，再继续往后推进：

1. S-mode + `SUM=1` 对 `U=1,X=1` 页面取指必须 fault
2. M-mode + `MPRV=1` + `MPP=S` 时 data access 必须受 PMP 限制
3. RV64 写 `pmpcfg2` 后 `entry 8..15` 真正生效
4. `L=1` 后写 `pmpcfg[i]` / `pmpaddr[i]` 被忽略
5. `L=1` + `TOR` 时写 `pmpaddr[i-1]` 被忽略
6. `lr.w` 在 “可读不可写” 页面上表现为 load fault，而不是 store/AMO fault
7. reset 后 TLB / PMP 状态清空
8. 跨 PMP region 的 8-byte access 必须 fault
9. `Bus::read` / `read_ram` / `load_ram` 对超大 `size` 或溢出边界输入返回错误，不 panic
10. `Ram::read` / `write` / `load` 越界时返回 `Err`，不 panic

## 总体判断

这轮实现已经把 memory subsystem 的主体框架搭起来了，方向没有偏。

但如果目标是 “按 `MEM_PLAN` 正确实现”，那么现在还差最后一轮 spec-level 收口。优先级上建议先修：

1. `SUM` / fetch 语义
2. `MPRV` 下 final PMP check
3. RV64 PMP CSR layout
4. PMP lock 语义
5. LR trap class
6. device layer 的 overflow-safe / non-panicking contract

前 5 个修完，行为层面才算收口；第 6 个修完，device layer 的 API 才算真正稳。
