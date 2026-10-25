# rCore ch9 API 实验：UART、virtio-blk 与 virtio-gpu 驱动

## 实验内容

本实验基于 `ch9-api` 分支，要求同学借助 AI 完成 UART 串口、virtio-blk 块设备与 virtio-gpu 图形设备三个驱动。完成后的驱动应支持串口中断接收与阻塞读取、块请求提交与完成通知，以及帧缓冲初始化、访问和显示刷新。

教材使用 [《rCore-Tutorial-Book-v3》第九章：I/O 设备管理](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter9/index.html)，完整参考实现位于 [ch9 分支](../../tree/ch9)。进程与线程、调度器、同步对象、文件系统、系统调用、中断分发、PLIC 及 VirtIO HAL 已经提供。实验只修改下表中的三个文件，共完成八处 TODO。

| 实现文件 | 待实现接口 | 数量 |
| --- | --- | --- |
| [os/src/drivers/chardev/ns16550a.rs](os/src/drivers/chardev/ns16550a.rs) | `CharDevice` 实现中的 `read`、`handle_irq` | 2 |
| [os/src/drivers/block/virtio_blk.rs](os/src/drivers/block/virtio_blk.rs) | `BlockDevice` 实现中的 `read_block`、`write_block`、`handle_irq` | 3 |
| [os/src/drivers/gpu/mod.rs](os/src/drivers/gpu/mod.rs) | `VirtIOGpuWrapper::new`，`GpuDevice` 实现中的 `get_framebuffer`、`flush` | 3 |

以下按接口给出 `description`、签名、输入、输出与关键约束。实验沿用本分支的单核 QEMU 环境、现有设备地址和有效请求约定；不扩展其他设备，也不设计新的公共 API。

## 实验要求

1. **代码修改范围**：仅允许补全上述三个文件中的八处 TODO，可在这三个文件内添加私有辅助函数。保持已有类型、字段、接口名称、可见性、参数和返回类型，保留已提供的构造器、原始寄存器操作、初始化与输出、光标辅助函数及已有日志方式。不能修改其他代码文件、依赖、构建配置或测试文件；可以新增实验报告等说明文档。

2. **静态分析与动态跟踪**：阅读 `ch9` 分支的三个驱动参考实现，按照 [ch9 源代码分析与动态跟踪文档](../../blob/ch9/rcore-ch9-analyze.md) 完成分析与 GDB 操作。观察 UART 接收缓冲与等待队列、块请求与完成通知的 token、GPU 帧缓冲地址和显示结果。跟踪在完整的 `ch9` 参考实现上进行，关联模块只分析必要的调用关系。报告记录实际断点、命令、关键状态与结论，可附必要代码片段或链接。

3. **独立实现与对比**：依据本文契约独立完成八处接口，并按“运行与验收”小节运行既有程序。在报告中对比自己的实现与参考实现，说明串口等待与唤醒、块请求等待登记与响应检查、GPU 显存保存与刷新方式的异同及原因。

4. **主要问题与解决思路**：总结主要问题的现象、原因、排查依据、解决过程与结果。关注等待登记与中断之间的时序、调度前释放设备借用、请求缓冲的生命周期和显存访问等问题；未解决的问题如实记录。Docker 环境中的图形显示问题也按本文要求记录。

实验报告采用 Markdown 格式，保存到仓库根目录的 `reports/lab9.md`，至少包含静态分析与动态跟踪、独立实现对比、主要问题与解决思路三部分，并保留相关 GDB 日志和 AI 交互记录，随实验代码提交。

## 提供的数据结构与支持接口

### UART

```rust
struct NS16550aInner {
    ns16550a: NS16550aRaw,
    read_buffer: VecDeque<u8>,
}

pub struct NS16550a<const BASE_ADDR: usize> {
    inner: UPIntrFreeCell<NS16550aInner>,
    condvar: Condvar,
}
```

`read_buffer` 按 FIFO 顺序保存中断处理收到的字符，`condvar` 保存等待输入的线程。已提供的 `NS16550aRaw::read() -> Option<u8>` 检查接收寄存器：有字符时返回 `Some(ch)`，当前没有字符时返回 `None`；它本身不阻塞线程。

### virtio-blk

```rust
pub struct VirtIOBlock {
    virtio_blk: UPIntrFreeCell<VirtIOBlk<'static, VirtioHal>>,
    condvars: BTreeMap<u16, Condvar>,
}
```

构造器已经建立 VirtIO 块设备，并为请求队列中的每个 token 创建条件变量。底层 `read_block_nb`、`write_block_nb` 提交请求并返回 token，`pop_used` 取出已经完成的 token；等待和唤醒必须使用同一个 token 对应的条件变量。

全局 `DEV_NON_BLOCKING_ACCESS` 控制两种访问模式：启动阶段为 `false`，使用底层同步轮询接口；任务开始运行前设为 `true`，之后使用请求提交、阻塞等待和完成中断。只读取该标志决定路径，不在驱动中改变它。

### virtio-gpu

```rust
pub struct VirtIOGpuWrapper {
    gpu: UPIntrFreeCell<VirtIOGpu<'static, VirtioHal>>,
    fb: &'static [u8],
}
```

`gpu` 持有设备及其 DMA 资源，`fb` 保存该设备帧缓冲的视图。`VirtIOGpu::setup_framebuffer()` 建立帧缓冲并返回可变字节切片，`VirtIOGpu::flush()` 提交显示更新。私有辅助函数 `setup_cursor(gpu: &mut VirtIOGpu<'static, VirtioHal>)` 已提供，负责光标素材转换与初始化；在构造设备时调用即可。

### 受保护访问、等待与调度

| 已提供接口 | 行为 |
| --- | --- |
| `UPIntrFreeCell::exclusive_access()` | 关中断并取得内部可变借用；守卫释放时结束借用并按嵌套情况恢复中断状态 |
| `UPIntrFreeCell::exclusive_session(...)` | 在受保护的内部可变借用范围内执行闭包，返回时释放借用 |
| `Condvar::wait_no_sched() -> *mut TaskContext` | 将当前线程加入等待队列、设为阻塞，返回其任务上下文指针；此时尚未切换线程 |
| `Condvar::signal()` | 唤醒等待队首的一个线程，使其进入就绪队列；没有等待者时直接返回，不立即调度 |
| `schedule(task_cx_ptr: *mut TaskContext)` | 切换到调度器；等待线程被唤醒并再次获得 CPU 后，从调用处继续执行 |

UART 的“检查接收缓冲、登记等待”和块设备的“提交请求、登记等待”必须分别处于同一次关中断的受保护访问内。这样，中断处理不会在等待者尚未登记时发出完成通知。调用 `schedule` 前必须释放设备及其他内部状态的借用，等待期间保持请求使用的内存有效。中断处理函数只处理已经到达的数据或完成项，不等待新事件、不主动切换线程。

## UART 接口

### `NS16550a::read`

description: 读取最早到达的一个字符；没有字符时阻塞当前线程，收到通知后重新检查缓冲。

```rust
fn read(&self) -> u8;
```

- 输入：UART 设备引用；调用路径中存在可阻塞的当前线程。
- 输出：从接收缓冲队首取出的一个字节。
- 关键约束：
  - 在 `inner` 的受保护访问内检查缓冲，有数据时取出并返回。
  - 缓冲为空时，在同一受保护范围内调用 `condvar.wait_no_sched()` 登记等待。
  - 释放设备借用后调用 `schedule`；恢复后循环重新检查缓冲，不能把收到通知直接当作已经取得字符。
  - 从中断接收缓冲取字符，不通过反复轮询原始寄存器代替线程阻塞。

### `NS16550a::handle_irq`

description: 收取当前可读的全部串口字符，放入接收缓冲并通知等待者。

```rust
fn handle_irq(&self);
```

- 输入：收到串口中断的 UART 设备引用。
- 输出：无返回值；更新接收缓冲，必要时唤醒一个等待线程。
- 关键约束：
  - 在设备受保护访问内反复调用原始 `read`，直到返回 `None`，按接收顺序将字符追加到缓冲队尾。
  - 记录本次是否收到了字符；释放设备借用后，若收到至少一个字符，调用一次 `condvar.signal()`。
  - 没有新字符时不发通知；不等待下一个字符，也不在此处调用 `schedule`。

## virtio-blk 接口

### `VirtIOBlock::read_block`

description: 将指定块读入调用者的缓冲，完成后返回；根据当前阶段选择轮询或中断完成路径。

```rust
fn read_block(&self, block_id: usize, buf: &mut [u8]);
```

- 输入：有效的块号 `block_id`，以及符合既有块设备接口长度要求的目标缓冲 `buf`。
- 输出：无返回值；成功返回时，`buf` 已包含读出的块数据。
- 关键约束：
  - 读取 `DEV_NON_BLOCKING_ACCESS` 后释放其借用。为 `false` 时，在设备受保护访问内调用底层同步 `read_block` 并检查结果，不登记线程等待；启动阶段尚无当前任务可供阻塞。
  - 为 `true` 时，准备 `BlkResp::default()`，在设备受保护访问内调用 `read_block_nb` 提交请求，取得 token 后立即在对应条件变量上调用 `wait_no_sched`。
  - 结束设备借用后再 `schedule`，恢复后检查 `resp.status()` 为 `RespStatus::Ok`。
  - `buf` 和响应对象在请求完成前必须一直有效，不能让设备访问已经释放或失效的内存。底层非阻塞提交不改变本接口等待 I/O 完成后才返回的约定。
  - 沿用参考实现的错误处理：提交失败或完成响应异常时 panic，不伪造成功或加入新的错误返回接口。

### `VirtIOBlock::write_block`

description: 将调用者缓冲中的数据写入指定块，完成后返回。

```rust
fn write_block(&self, block_id: usize, buf: &[u8]);
```

- 输入：有效的块号 `block_id`，以及符合既有块设备接口长度要求的源缓冲 `buf`。
- 输出：无返回值；成功返回时，设备已完成本次写请求。
- 关键约束：
  - 与 `read_block` 一样读取访问模式：`false` 时调用底层同步 `write_block`，`true` 时使用 `write_block_nb` 提交请求。
  - 中断完成路径使用独立的响应对象，在同一次设备受保护访问中提交请求并按 token 登记等待；释放借用后调度，恢复后检查 `RespStatus::Ok`。
  - 等待期间源缓冲和响应对象保持有效，源数据不能提前修改或释放；不在提交请求后立即向调用者返回。
  - 沿用参考实现的失败处理，不引入重试协议或新的公共 API。

### `VirtIOBlock::handle_irq`

description: 消费当前所有已完成的块请求，按 token 唤醒对应等待者。

```rust
fn handle_irq(&self);
```

- 输入：收到完成中断的块设备引用。
- 输出：无返回值；已完成请求对应的线程进入就绪队列。
- 关键约束：
  - 在设备受保护访问内反复调用 `pop_used()`，每取得一个 `Ok(token)`，就在 `condvars` 中找到该 token 的条件变量并调用 `signal()`。
  - 直到没有可取出的完成项才结束，不能假设一次中断只对应一个请求，也不能唤醒与完成 token 无关的等待者。
  - 不等待尚未完成的请求，不主动调度线程；响应状态由恢复后的读写接口检查。

## virtio-gpu 接口

### `VirtIOGpuWrapper::new`

description: 创建 GPU 设备、初始化帧缓冲和已有光标，保存设备及其显存视图。

```rust
pub fn new() -> Self;
```

- 输入：无显式参数，使用已有 `VIRTIO7` 设备地址和 `VirtioHal`。
- 输出：初始化完成的 `VirtIOGpuWrapper`。
- 关键约束：
  - 通过 `VIRTIO7` 对应的 `VirtIOHeader` 创建底层 `VirtIOGpu`，调用 `setup_framebuffer` 建立帧缓冲。
  - 记录返回切片的原始地址和长度，保存对同一段显存的视图。切片长度由设备返回，不硬编码分辨率或缓冲大小。
  - 调用已提供的 `setup_cursor` 完成光标初始化，将设备保存在 `gpu` 中，并保存 `fb`。不得提前销毁设备、释放 DMA 页或把显存复制到另一个像素缓冲代替原显存。
  - 沿用现有字段与单核使用约定，处理好底层方法借用与保存显存视图的关系；不能返回临时变量的引用。设备初始化失败时 panic。

### `VirtIOGpuWrapper::get_framebuffer`

description: 返回已经建立的设备帧缓冲，供现有图形调用路径访问。

```rust
fn get_framebuffer(&self) -> &mut [u8];
```

- 输入：GPU 设备引用。
- 输出：可变字节切片，地址和长度与 `self.fb` 一致。
- 关键约束：返回同一块设备显存的视图，不重新初始化、清零、分配、复制或释放显存，不在此处另建用户映射。沿用已有调用约定及签名；显存资源的生命周期由对象内保存的底层 GPU 驱动维持。

### `VirtIOGpuWrapper::flush`

description: 将当前帧缓冲内容提交给 GPU，更新显示。

```rust
fn flush(&self);
```

- 输入：GPU 设备引用；调用者已通过既有帧缓冲访问路径写入像素。
- 输出：无返回值；底层刷新请求成功完成。
- 关键约束：在 `gpu` 的受保护访问内调用底层 `flush` 并检查结果，失败时 panic。不重新创建帧缓冲，不在此处实现绘图算法。

## 已提供的实现与实验边界

- UART 的寄存器布局、原始寄存器访问、`NS16550aRaw` 全部方法、`NS16550a::new`、`read_buffer_is_empty`，以及 `CharDevice` 实现中的 `init`、`write` 均已提供。
- `VirtIOBlock::new`、token 到条件变量的映射、VirtIO HAL 和底层驱动库已提供。无需实现 VirtIO 协议或 DMA 分配器，不修改文件系统与块缓存。
- GPU 全局设备、现有类型与字段、光标素材和 `setup_cursor` 已提供。图形系统调用、用户地址映射、应用绘图、键鼠输入等保持原样。
- 条件变量、调度器、中断入口与 PLIC 分发只用于理解调用衔接，不属于本次重构范围。

## 运行与验收

先按 [README 的工具链说明](./README.md) 准备本章环境。骨架可执行 `make build` 完成编译；尚未实现的接口运行到 `todo!()` 时会 panic，这是待完成状态。补全八处接口后，从仓库根目录运行：

```bash
cd os
make run GUI=on
```

在 QEMU 串口终端的 shell 中依次运行以下已有程序，不新增测试程序：

```text
filetest_simple
huge_write
gui_shape
```

| 验收内容 | 预期结果 |
| --- | --- |
| UART 输入与等待 | shell 能回显输入字符并接收完整命令；没有输入时等待字符，输入后继续处理。结合参考实现跟踪与代码分析说明阻塞、唤醒过程 |
| `filetest_simple` | 文件写入与读回一致，输出 `file_test passed!` |
| `huge_write` | 完成 1 MiB 文件写入，输出耗时和速度并返回 shell；不要求固定性能数值 |
| `gui_shape` | 图形界面显示蓝色圆形、红色方框和绿色三角形 |

>在 Docker 环境中，GPU 图形可能无法正常显示。遇到此类问题时，请结合宿主机和容器的实际环境，借助 AI 排查并解决显示问题，在 `reports/lab9.md` 中记录环境、问题现象、AI 辅助排查过程、采用的解决方法及最终显示效果，并保留相关 AI 交互记录。

本实验的功能验收限于上述驱动相关操作。GDB 跟踪用于分析参考实现；不要求增加其他设备或模块的测试。
