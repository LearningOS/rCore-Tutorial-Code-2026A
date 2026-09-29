# rCore ch3 API 实验：任务管理

## 实验内容

本实验基于 `ch3-api` 分支，要求同学借助 AI 完成第三章的任务管理模块，使多个静态加载的应用能够在单核环境中轮流执行。需要实现的功能包括任务管理状态的初始化、首个任务的启动、当前任务的暂停与恢复，以及当前任务退出后的调度。

实验提供必要的数据结构和对外接口签名。学生需要完成 [os/src/task/mod.rs](os/src/task/mod.rs) 中的四处 TODO：`TASK_MANAGER` 的初始化表达式，以及 `run_first_task`、`suspend_current_and_run_next`、`exit_current_and_run_next` 三个公共函数。这三个公共函数是其他内核模块调用任务调度功能的固定入口，函数名称、可见性、参数和返回类型均应保持原样。内部辅助函数的名称、签名和组织方式由学生自行设计。

本文沿用 [rCore-Tutorial-v3 接口文档](https://github.com/rcore-os/rCore-Tutorial-v3-api-doc/blob/main/rCore-Tutorial-v3.md) 按模块和接口组织 `description` 与代码声明的形式。以下职责、输入、输出和关键约束以本仓库 `ch3-api` 代码为准。

## 实验要求

1. **代码修改范围**：任务管理实现仅允许修改 [os/src/task/mod.rs](os/src/task/mod.rs) 和 [os/src/task/task.rs](os/src/task/task.rs)，完成 `TASK_MANAGER` 初始化以及 `run_first_task()`、`suspend_current_and_run_next()`、`exit_current_and_run_next()` 四处 TODO。内部辅助函数可自行设计，保留数据结构、对外接口及已提供的系统调用计数实现。若启动栈不足，允许按下文提示调整 [os/src/entry.asm](os/src/entry.asm) 中 `boot_stack` 的预留容量。除此之外，不得修改其他代码文件、汇编、链接脚本、构建配置或测试文件；可以新增实验报告等说明文档。

2. **静态分析与动态跟踪**：阅读 `ch3` 的参考实现，使用 GDB 跟踪首个任务启动、当前任务暂停与恢复、任务退出后的调度，分析任务状态、当前任务编号和上下文切换的对应关系。操作可参考 [ch3 源代码分析与动态跟踪文档](../../blob/ch3/rcore-ch3-analyze.md)。报告应结合源代码说明调用顺序及其原因，记录关键断点、使用的 GDB 命令、观察到的状态和分析结论；可附必要的代码片段或源码链接。

3. **独立实现与对比**：依据本文接口契约完成任务管理，并按“运行与验收”小节执行验收。在报告中比较自己的实现与参考实现的相同点和不同点，说明初始化范围、轮转选择、状态变化、上下文恢复和内部函数组织的处理，以及采用相关实现方式的原因；不能只列出代码文本的差异。参考代码与本文约束存在差别时，应按接口契约实现并说明原因。

4. **主要问题与解决思路**：总结实验中遇到的主要问题，描述问题现象、原因分析、排查过程、解决思路及处理结果，结合代码、GDB 跟踪记录或运行输出说明依据。尚未解决的问题也应如实记录。

实验报告采用 Markdown（`.md`）格式，统一保存到仓库根目录下的 `reports/lab3.md`，至少包含静态分析与动态跟踪、独立实现对比、主要问题与解决思路三部分，并随实验代码提交。GDB 动态跟踪用于参考实现分析，完成后的功能验收仍使用本文规定的运行命令。

## os::task::task

description: 该子模块定义任务控制块和任务状态，源码位于 [os/src/task/task.rs](os/src/task/task.rs)。这些数据结构已经提供，用来统一任务管理、上下文切换和系统调用统计所使用的任务表示。

### TaskStatus

description: `TaskStatus` 定义本章任务可能处于的四种状态。初始化与三个公共接口需要按照各自契约维护这些状态。

```rust
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}
```

| 状态 | 含义 |
| --- | --- |
| `UnInit` | 尚未初始化的任务槽位 |
| `Ready` | 已具备执行条件，等待调度 |
| `Running` | 当前正在执行的任务，包括该任务正在执行的内核处理过程 |
| `Exited` | 已结束执行的任务，该状态为终态 |

### TaskControlBlock

description: `TaskControlBlock` 保存一个静态加载应用对应的任务信息。`task_status` 表示任务的生命周期状态，`task_cx` 保存任务在内核中启动或恢复所需的寄存器上下文，`syscall_counts` 保存该任务按系统调用编号索引的调用次数。任务编号由控制块所在的数组索引确定。

```rust
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub syscall_counts: [usize; MAX_SYSCALL_NUM],
}
```

## os::task

description: `os::task` 管理静态加载应用的执行状态，并为内核启动、系统调用和异常处理提供任务调度能力。一个应用对应一个任务，任务编号就是它在任务控制块数组中的索引。模块通过全局任务管理器 `TASK_MANAGER` 维护实际应用数量、各任务的控制块以及当前任务编号。

任务管理同时涉及执行资格和执行上下文。任务状态描述应用是否已初始化、是否可以调度、是否正在运行或是否已经退出；任务上下文描述应用在内核中开始或恢复执行所需的寄存器状态。调度结果必须同时满足任务状态和上下文的约定，使被暂停的任务能够继续原来的执行。

三个公共接口与既有内核代码的关系如下：

| 公共接口 | 调用方 | 调用目的 |
| --- | --- | --- |
| `run_first_task()` | [rust_main](os/src/main.rs) | 应用加载及中断初始化完成后，启动首个应用 |
| `suspend_current_and_run_next()` | [sys_yield](os/src/syscall/process.rs)、[时钟中断处理](os/src/trap/mod.rs) | 支持应用主动让出 CPU 和时钟中断引起的抢占 |
| `exit_current_and_run_next()` | [sys_exit](os/src/syscall/process.rs)、[应用异常处理](os/src/trap/mod.rs) | 支持应用正常退出，以及访问异常、非法指令导致的任务终止 |

### TaskManager 与 TaskManagerInner

description: `TaskManager` 是全局任务管理器的数据类型，其中 `num_app` 表示实际加载的应用数量，`inner` 封装需要在运行期间修改的调度状态。`TaskManagerInner` 包含固定容量的任务控制块数组和当前任务编号。这两个结构体的定义已经提供，学生可以围绕它们自行设计内部方法。

```rust
pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

pub struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}
```

`MAX_APP_NUM` 表示数组容量，`num_app` 表示实际参与本次运行的应用数量。`tasks` 中的有效任务通过数组索引标识，`current_task` 使用相同的编号体系。`TASK_MANAGER` 的初始化需要建立这些字段之间的对应关系，三个公共接口需要在运行期间维持它们的一致性。

### TASK_MANAGER

description: `TASK_MANAGER` 是整个内核共用的任务管理器。其初始化需要根据已经加载的应用，形成完整的任务管理状态，使每个有效任务都具备首次运行的条件。初始化完成后，三个公共调度接口以及已提供的系统调用统计接口都访问这个管理器。

本实现点位于 `lazy_static!` 中的初始化表达式。该表达式在全局管理器首次被访问时求值，产生由 `TASK_MANAGER` 持有的 `TaskManager`。初始化结果描述各应用准备运行的状态，首个应用的实际启动由 `run_first_task()` 完成。是否定义构造函数，以及如何组织初始化所需的内部代码，由学生自行决定。

> **启动栈提示**：`TASK_MANAGER` 初始化在 `boot_stack` 上执行。任务数组中的系统调用计数器合计约占 51 KiB，构造和返回任务管理器时还可能产生临时副本，尤其在 `MODE=debug` 下，可能导致启动栈溢出。当前 [os/src/entry.asm](os/src/entry.asm) 使用 `.space 4096 * 16` 预留 64 KiB 启动栈；若容量不足，可按需要增大该值，并在实验报告中说明原因。`config.rs` 中的 `KERNEL_STACK_SIZE` 控制任务内核栈，调整它不会增大 `boot_stack`。

```rust
lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        todo!("task::TASK_MANAGER")
    };
}
```

**输入**

没有显式参数。输入来自 `loader` 提供的已加载应用信息，包括实际应用数量、应用编号及各应用的初始执行环境。`MAX_APP_NUM` 和 `MAX_SYSCALL_NUM` 分别约定任务数组和每个任务的系统调用计数数组的容量。应用代码的加载由已有启动框架完成。

**输出**

初始化表达式产生一个可供任务启动与调度使用的 `TaskManager`。有效任务均处于 `Ready` 状态，拥有与自身应用对应的首次运行上下文；`current_task` 的初值为 `0`。此时任务管理状态已经准备就绪，应用尚未由任务模块启动。

**关键约束**

- `num_app` 必须等于实际加载的应用数量，满足 `1 <= num_app <= MAX_APP_NUM`。
- `tasks` 中索引位于 `0..num_app` 的控制块分别对应同编号应用；这些任务为 `Ready`，其上下文应能支撑首次进入相应应用。
- 其余任务槽位保持 `UnInit`；首次启动前，`current_task` 为 `0`。
- 每个任务的 `syscall_counts` 数组初值均为零，各任务的计数存储相互独立。
- 初始化结果保留已提供的数据结构定义，与公共调度接口及已实现的系统调用统计接口使用同一份管理状态。

### run_first_task

description: `run_first_task` 是 task 模块提供给内核启动代码的入口，负责将执行权从内核启动阶段交给编号为 `0` 的应用。已有的 `rust_main` 在应用加载、陷阱处理初始化和时钟中断设置完成后调用它。该接口需要使首个应用基于初始化阶段准备的执行环境开始运行，并使全局任务管理状态反映这一事实。

该函数承接 `TASK_MANAGER` 初始化的结果：初始化确定有哪些任务以及它们的初始上下文，本接口使首个任务实际获得 CPU。接口执行成功后，控制流进入首个应用，内核启动代码不再继续执行。

```rust
pub fn run_first_task() {
    todo!("task::run_first_task")
}
```

**输入**

没有显式参数。依赖全局 `TASK_MANAGER` 和已经加载的应用；这是本次内核运行中首次启动任务的入口。首个任务的初始上下文应与编号为 `0` 的应用对应。

**输出**

编号为 `0` 的应用开始执行，任务状态为 `Running`，`current_task` 为 `0`。函数签名中的返回类型隐含为 `()`，但按接口契约，正常执行不会返回 `rust_main` 的调用点。

**关键约束**

- 保留公共无参函数的签名，供现有启动代码直接调用。
- 本接口用于首次启动，选择的任务编号固定为 `0`。
- 首个任务的执行环境必须与初始化结果一致，使其能够正确进入对应应用。
- 启动后的 `current_task`、任务状态和实际执行任务保持一致，并遵守模块共同约束中的上下文有效性与借用要求。

### suspend_current_and_run_next

description: `suspend_current_and_run_next` 暂停当前任务并将执行权交给按轮转规则选中的就绪任务。它同时服务于两类调用：应用通过 `sys_yield` 主动让出 CPU，以及时钟中断触发的抢占。无论由哪一种原因进入该接口，当前任务都保留后续继续执行的资格，其暂停时的执行进度必须能够恢复。

一次调用可能跨越其他任务的执行过程。原任务再次获得 CPU 时，应恢复本次调用所对应的内核执行流，随后该函数向原调用方返回。这样，系统调用或陷阱处理可以继续完成原任务的处理过程，最终让应用继续运行。接口必须支持同一任务多次暂停和恢复。

```rust
pub fn suspend_current_and_run_next() {
    todo!("task::suspend_current_and_run_next")
}
```

**输入**

没有显式参数。`TASK_MANAGER` 中的 `current_task` 指向调用发生时正在运行的任务，其状态为 `Running`。任务数组提供当前任务与其他有效任务的状态及上下文，作为本次调度所需的管理信息。

**输出**

执行权交给按模块约定选中的就绪任务。原任务在等待再次执行期间保持 `Ready`，并保有可恢复的执行上下文。当原任务重新获得执行权时，本次调用返回 `()`，原调用方继续其已有的内核控制流。

**关键约束**

- 主动让出 CPU 和时钟抢占共用这一固定入口，两种情况下都应保持任务可恢复。
- 后继任务的选择遵循有效任务编号范围内的轮转顺序，等待调度的任务状态为 `Ready`。
- 实际运行任务的编号、`current_task` 和 `Running` 状态保持一致。
- 恢复执行延续原任务已有的上下文和执行进度，各任务的系统调用累计计数保持正确。
- 上下文切换时不持有任务管理器内部状态的动态借用，上下文指针在使用期间保持有效。

### exit_current_and_run_next

description: `exit_current_and_run_next` 是当前任务终止执行的调度入口。它由 `sys_exit` 处理正常退出时调用，也由陷阱处理代码在终止发生异常的应用时调用。接口需要结束当前任务的运行资格，使其进入 `Exited` 状态，并让后续选中的就绪任务继续运行。

任务退出意味着本次执行生命周期结束。已经退出的任务不再恢复其系统调用或异常处理现场，也不再回到应用继续执行。其他就绪任务仍通过统一的任务管理状态参与轮转调度。该接口使用当前任务作为操作对象，不接收任务编号或退出码参数。

```rust
pub fn exit_current_and_run_next() {
    todo!("task::exit_current_and_run_next")
}
```

**输入**

没有显式参数。`TASK_MANAGER` 中的 `current_task` 指向需要终止的当前任务，调用发生时该任务的状态为 `Running`。退出原因由已有系统调用或异常处理代码处理，本接口负责任务状态和调度语义。

**输出**

当前任务结束执行并处于 `Exited` 状态，执行权交给按模块约定选中的就绪任务。函数签名保留隐含的 `()` 返回类型；正常退出不会返回原任务的调用点，也不会恢复该任务的应用执行。

**关键约束**

- 已退出任务的状态为 `Exited`，后续调度不得再次选中它。
- 后继任务的选择遵循模块的轮转约定，实际执行任务与 `current_task`、`Running` 状态一致。
- 其他有效任务的执行上下文和各自的系统调用累计计数保持可用。
- 交接执行权时遵守模块共同约束中的上下文有效性和动态借用要求。

## 已提供的配套功能

以下代码已完整提供，可作为任务管理实现的依赖。学生保留这些实现和现有调用关系。

| 代码位置 | 已提供内容 |
| --- | --- |
| [os/src/loader.rs](os/src/loader.rs) | 应用加载、应用数量查询，以及各应用的初始陷阱上下文与栈支持 |
| [os/src/task/context.rs](os/src/task/context.rs) | `TaskContext` 定义、`zero_init()` 和 `goto_restore(kstack_ptr)` |
| [os/src/task/switch.rs](os/src/task/switch.rs)、[os/src/task/switch.S](os/src/task/switch.S) | `__switch` 的 Rust 声明及寄存器上下文切换的汇编实现 |
| [os/src/sync/up.rs](os/src/sync/up.rs) | 单核环境下的 `UPSafeCell` 与动态借用访问 |
| [os/src/timer.rs](os/src/timer.rs) | 时间查询与 `set_next_trigger()` 的完整实现 |
| [os/src/syscall](os/src/syscall)、[os/src/trap](os/src/trap) | 系统调用分发、异常与中断处理，以及完整的 `sys_trace` |
| [os/src/task/mod.rs](os/src/task/mod.rs) | 已实现的 `record_current_syscall()` 和 `current_syscall_count()` |

`record_current_syscall()` 和 `current_syscall_count()` 继续由既有系统调用代码使用。它们依赖 `TASK_MANAGER` 中当前任务编号与控制块数组的一致性，并直接访问对应任务的 `syscall_counts`，因此任务管理的实现需要与这些字段约定兼容。

启动汇编、陷阱汇编和链接脚本均按仓库原样提供。任务管理的实现范围是 `os/src/task/mod.rs` 与 `os/src/task/task.rs`；当前四处 TODO 均位于前者。启动栈不足时，仅允许额外调整 `os/src/entry.asm` 中 `boot_stack` 的预留容量。数据结构和对外接口的约定保持不变，内部辅助函数可以自由定义。

## 运行与验收

验收使用仓库现有的 `make run`。在运行环境和已有 `user` 测试目录准备好后，从仓库根目录执行：

```bash
cd os
make run CHAPTER=3 BASE=2
```

`CHAPTER=3` 显式指定第三章，适用于带有后缀的 `ch3-api` 分支名。`BASE=2` 使用现有测试目录中的第二、三章基础与编程测试应用，包含任务切换、时间查询和 `sys_trace` 相关用例。

完成后的实现应能够运行已有应用并通过应用中的断言和结果检查，包括 `Test write A OK!`、`Test write B OK!`、`Test write C OK!`、`Test sleep OK!`、`Test sleep1 passed!` 和 `Test trace OK!`。验收以这些既有应用的实际运行结果为准。
