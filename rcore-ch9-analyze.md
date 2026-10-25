# rCore ch9：源代码分析与 GDB 动态跟踪

阅读 `ch9` 的参考实现，结合 [《rCore-Tutorial-Book-v3》第九章：I/O 设备管理](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter9/index.html)，分析 UART 串口、virtio-blk 和 virtio-gpu 三个驱动。本实验保留完整参考代码，只要求源码分析、运行观察与实验报告，不修改代码。

## 1. 使用说明

先完成 [README 中的工具链准备](./README.md)。在仓库根目录执行以下命令，准备 release 用户程序和文件系统镜像，再单独构建未优化、带调试信息的 debug 内核：

```bash
cd os
make build
RUSTFLAGS='-C link-arg=-Tsrc/linker-qemu.ld -C force-frame-pointers=yes' cargo build
rust-objcopy --binary-architecture=riscv64 --strip-all -O binary \
  target/riscv64gc-unknown-none-elf/debug/os \
  target/riscv64gc-unknown-none-elf/debug/os.bin
```

这些命令仅用于本节分析；日常运行继续使用 `make run` 或 `make run GUI=on`。分析期间使用已经生成的用户镜像；如需重新构建，先退出 QEMU。

先按下表阅读源码：

| 阅读位置 | 分析内容 |
| --- | --- |
| [drivers/chardev/ns16550a.rs](os/src/drivers/chardev/ns16550a.rs) | `NS16550aRaw` 的初始化、寄存器访问和轮询输出；`NS16550a` 的接收缓冲、阻塞读取与中断唤醒。 |
| [drivers/block/virtio_blk.rs](os/src/drivers/block/virtio_blk.rs) | 设备和条件变量表的初始化；`read_block`、`write_block` 的两种访问路径；请求 token 与 `handle_irq` 完成通知的对应关系。 |
| [drivers/gpu/mod.rs](os/src/drivers/gpu/mod.rs) | `VirtIOGpuWrapper::new` 建立并保存帧缓冲；`get_framebuffer` 提供显存访问；`flush` 提交显示更新。光标素材转换不单列分析任务。 |

关联代码只阅读必要的调用关系：标准输入如何调用 UART，文件系统如何发起块请求，图形系统调用如何取得显存并请求刷新。中断入口、PLIC、条件变量、调度器、页表和第三方驱动库不单独展开；DMA 只解释设备访问内存与地址转换的作用。

## 2. 操作样例

下面三个样例分别从一次新的 QEMU 调试会话开始，构建只需执行一次。

终端一从仓库根目录执行：

```bash
cd os
make -o build gdbserver MODE=debug GUI=on \
  FS_IMG=../user/target/riscv64gc-unknown-none-elf/release/fs.img
```

`-o build` 直接使用前面生成的内核与镜像，`FS_IMG` 指定 release 用户镜像。图形显示的环境问题按 [README 中的说明](./README.md) 处理；实验程序名称和串口字符仍在终端一输入。

终端二从仓库根目录执行：

```bash
mkdir -p reports
cd os
riscv64-unknown-elf-gdb -nx -q target/riscv64gc-unknown-none-elf/debug/os
```

在 GDB 中连接并记录操作：

```gdb
set pagination off
set architecture riscv:rv64
set language c
set logging file ../reports/ch9-gdb.log
set logging overwrite off
set logging enabled on
target remote localhost:1234
```

以下使用源码行断点，停在局部变量已经初始化的位置。用 `p` 观察内存变量，用 `next` 越过配套接口；不要通过 GDB 调用驱动方法或读取有副作用的 MMIO 寄存器来查看状态。

每组样例结束后，在 GDB 中按 `Ctrl+C` 暂停，再执行 `monitor quit` 退出 QEMU、`quit` 退出 GDB。下一组重新执行上述两个终端的启动命令和连接设置。退出 QEMU 后出现远程连接断开提示是正常的。

### UART：等待字符与中断接收

在启动前设置等待位置的断点：

```gdb
tbreak src/drivers/chardev/ns16550a.rs:164
continue
bt 5
p self->inner.inner.value.value.read_buffer.len
p self->condvar.inner.inner.value.value.wait_queue.len
next
p self->condvar.inner.inner.value.value.wait_queue.len
next
tbreak src/drivers/chardev/ns16550a.rs:182
continue
```

第一次暂停时，shell 正在读取标准输入，接收缓冲为空。比较 `wait_no_sched` 前后的等待队列长度，说明为什么登记等待后，还要释放设备借用再调度。

最后一次 `continue` 后，在终端一输入一个字符。GDB 会停在接收中断已处理完本批字符的位置：

```gdb
p count
p self->inner.inner.value.value.read_buffer.len
p self->condvar.inner.inner.value.value.wait_queue.len
next
next
p self->condvar.inner.inner.value.value.wait_queue.len
continue
```

观察本批接收数量、缓冲长度以及通知前后的等待队列变化。继续后确认字符在 shell 中回显，结合源码解释等待线程恢复后如何取出字符。UART 的输出路径只需静态分析其等待发送寄存器可写的过程。

### virtio-blk：提交请求与完成通知

重新启动调试会话，在写请求提交后设置断点：

```gdb
tbreak src/drivers/block/virtio_blk.rs:45
continue
```

出现 shell 后，在终端一输入 `huge_write`。命中断点时，驱动已提交一个写请求，正准备登记等待：

```gdb
p block_id
p token
bt 5
tbreak src/drivers/block/virtio_blk.rs:63
tbreak src/drivers/block/virtio_blk.rs:49
continue
```

下一次停在完成中断处理中的 `signal` 前，记录完成的 token：

```gdb
p token
bt 5
continue
p nb
p resp
continue
```

比较提交和完成阶段的 token；最后一次暂停位于写请求恢复后的响应检查处，观察访问模式和响应状态。继续运行，确认 `huge_write` 完成并回到 shell，不要求固定耗时或吞吐量。

结合 `read_block` 的对称流程解释读请求，再运行 `filetest_simple` 观察文件读写结果。文件缓存可能使一次文件操作不产生设备请求；请求提交、完成中断和线程恢复是三个执行阶段，并非同一条连续调用栈。底层 `*_block_nb` 非阻塞提交请求，外层块读写接口仍等待完成后返回。

### virtio-gpu：帧缓冲初始化与刷新

重新启动调试会话，在帧缓冲已经建立的位置暂停：

```gdb
tbreak src/drivers/gpu/mod.rs:34
continue
p len
p/x ptr
p fb
tbreak src/drivers/gpu/mod.rs:62
continue
```

记录驱动取得的显存地址与长度。出现 shell 后，在终端一输入 `gui_shape`，观察应用获取帧缓冲和刷新显示的入口：

```gdb
p self->fb
bt 4
tbreak src/drivers/gpu/mod.rs:58
continue
p self->fb
bt 4
continue
```

比较初始化与运行阶段的显存地址、长度，并从调用栈识别两个图形系统调用。继续后，在图形界面中观察蓝色圆形、红色方框和绿色三角形。结合源码说明应用写入像素与驱动刷新显示的关系，不要求逐像素跟踪。

> 在 Docker 环境中，GPU 图形可能无法正常显示。遇到此类问题时，请结合宿主机和容器的实际环境，借助 AI 排查并解决显示问题。在实验报告 `reports/lab9.md` 中说明运行环境、问题现象、AI 辅助排查过程、采用的解决方法及最终显示效果，并保留相关 AI 交互记录。

## 3. 需要追踪的调用链

| 调用链 | 触发方式 | 需要解释的状态变化 |
| --- | --- | --- |
| `Stdin::read` → `NS16550a::read` → `wait_no_sched` → `schedule` | shell 等待输入 | 接收缓冲为空，登记等待并阻塞；切换前释放设备借用。 |
| 中断分发 → `NS16550a::handle_irq` → 接收缓冲入队 → `signal`；随后恢复 `read` | 在串口终端输入字符 | 字符从设备进入缓冲，等待者被唤醒，恢复后重试读取。 |
| `VirtIOBlock::read_block/write_block` → 轮询访问或请求提交、等待 | 启动过程；运行文件操作程序 | 结合源码比较 `DEV_NON_BLOCKING_ACCESS` 的两种取值：启动阶段没有当前任务可供阻塞，运行阶段按请求 token 等待。 |
| `VirtIOBlock::handle_irq` → `pop_used` → 对应条件变量的 `signal`；随后恢复块读写 | 块请求完成 | 完成 token 对应等待者，恢复后检查响应状态。 |
| `VirtIOGpuWrapper::new` → `setup_framebuffer` → 保存帧缓冲 | 内核启动 | 帧缓冲的创建、地址、长度与保存方式。 |
| 图形系统调用 → `get_framebuffer`；应用写入像素；刷新系统调用 → `flush` | `gui_shape` | 应用访问已有显存，驱动提交显示更新；关联的用户映射只解释接口衔接。 |

三组动态样例记录关键状态，其余关联过程结合源码说明。重点分析等待登记与设备操作为何需要处于受保护的访问范围内，以及为什么调度之前必须释放内部借用。

## 4. 报告要求

报告保存到 `reports/lab9.md`，保留对应的 `reports/ch9-gdb.log`，至少包含：

1. **静态分析**：分别说明三个驱动的数据结构、主要接口与执行过程。比较 UART 的轮询输出与中断输入、块设备的两种访问路径，解释 GPU 显存访问与刷新之间的关系。可附必要代码片段或源码链接。
2. **动态跟踪**：记录实际使用的断点、命令、关键状态和运行结果。UART 记录缓冲及等待队列变化，块设备记录请求与完成的 token、响应状态，GPU 记录帧缓冲地址、长度和显示结果。区分实际观察与源码推导。
3. **主要问题与解决思路**：描述现象、排查依据、解决过程和结果，未解决的问题如实记录。

本阶段不要求修改驱动、完成 API 独立实现或扩展其他设备。GDB 跟踪用于分析，日常运行保持 README 中的原有命令。
