# rCore-Tutorial-Code

## ch9 实验说明

当前分支 `ch9` 提供第 9 章参考源代码，来自 [rCore-Tutorial-v3 的 ch9 分支](https://github.com/rcore-os/rCore-Tutorial-v3/tree/ch9)。

本章实验参考书为 [《rCore-Tutorial-Book-v3》第九章：I/O 设备管理](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter9/index.html)。请按 [ch9 源代码分析与动态跟踪文档](./rcore-ch9-analyze.md)，围绕 UART 串口、virtio-blk 和 virtio-gpu 三个驱动完成静态分析、GDB 动态跟踪和实验报告。报告保存到 `reports/lab9.md`。

## 工具链更新（ch9 必读）

> **开始 ch9 实验前，必须先安装或更新 stable 工具链。前面章节使用的 `nightly-2024-05-02` 无法直接构建本章。**

本章沿用上游的 Rust 2024 edition 和 `stable` 工具链，按以下命令完成准备。

在仓库根目录执行以下命令，安装或更新 stable，并准备所需组件和编译目标：

```bash
rustup toolchain install stable --profile minimal \
  --component rust-src --component llvm-tools --component rustfmt --component clippy \
  --target riscv64gc-unknown-none-elf
```

后续更新工具链可执行：

```bash
rustup update stable
```

仓库中的 [rust-toolchain.toml](./rust-toolchain.toml) 会自动选择本分支使用的工具链，无需修改全局默认工具链。可在仓库根目录检查当前版本：

```bash
rustc --version
cargo --version
```

构建还需要 `cargo-binutils`，尚未安装时执行：

```bash
cargo install cargo-binutils --locked
```

## 图形界面运行

如果在 Docker 环境中运行图形程序时无法正常显示，请借助 AI 排查并解决问题，在实验报告 `reports/lab9.md` 中说明问题现象、排查过程、解决方法和验证结果，并保留相关 AI 交互记录。

## 实验要求

从 [rCore 实验任务书](https://LearningOS.github.io/rCore-Tutorial-Guide/)的共 8 章中选择 5 章，完成以下任务：

1. **源代码分析与实验报告**：对所选 5 章的源代码进行**静态分析**和**跟踪分析**，撰写实验报告。需要分析的代码位于 `ch{$ID}` 分支。具体分析要求以课上说明为准。
2. **独立实现与对比**：从自己已分析的 5 章中选择 2 章进行独立实现，并在实验报告中比较自己的实现与参考实现的异同。需要独立实现的代码和说明文档位于 `ch{$ID}-api` 分支，完成所选的 2 章即可，可直接在对应分支提交。

（`$ID` 为章节编号，可选1-8）

- **ch9 实验**：参考 [《rCore-Tutorial-Book-v3》第九章：I/O 设备管理](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter9/index.html)，按 [ch9 分析文档](./rcore-ch9-analyze.md) 完成 UART、virtio-blk 和 virtio-gpu 三个驱动的分析与跟踪。

**代码提交必须包含开发过程的操作日志和 AI 交互日志**。请使用下方介绍的工具记录实验过程，并将日志随实验代码一起提交到实验仓库。

## 实验过程记录工具

课程过程记录：在仓库根目录运行 `python3 course.py`，先自动初始化 AI 会话归档，再打开实验与实时日志。安装要求、日志位置和 Codex 入口见 [实验过程记录说明](https://github.com/leeehh/rCore-Tutorial-Code-api/blob/main/docs/course-recording.md)。

AI过程记录：启动时默认使用 `auto`，可通过 `python3 course.py --agent codex` 指定客户端；也支持 `claude`、`cursor`、`vscode`、`copilot` 和 `all`。会话记录保存到 `.ai/agent-sessions/<agent>/`，文件名包含日期时间，格式为 JSONL。`.ai/agent-sessions/`、`.ai/events/` 和 `.ai/submissions/` 的全部内容须随实验代码一起提交。请同学们不要改动或删除这些记录，提交时会检查这些记录作为考核参考。详细说明见 [AI 会话归档说明](https://github.com/leeehh/rCore-Tutorial-Code-api/blob/main/docs/agent-session-archive.md)。

**Codex 首次使用需要信任 hooks**：安装完成后，在实验仓库根目录的终端运行 `codex`，进入后输入 `/hooks`，找到 `rcore-session-archive` 的 `Stop` 和 `SessionEnd`，分别审阅并选择 **Trust（信任）**。未信任时不会自动保存会话。使用 VS Code Codex 的同学完成后还需重载窗口并新建会话；更新插件后，如提示 hooks 发生变化，请重新审阅并信任。

记录功能与验证：[实验过程记录工具功能说明](https://github.com/leeehh/rCore-Tutorial-Code-api/blob/main/docs/course-monitor-report.md)。main 和各实验分支均提供 `course.py` 及完整运行依赖；安装一次后，切换到 `ch1`–`ch8` 仍会记录。实验分支可使用 `git course logs` 查看日志，使用 `git agent-plugins auto` 再次配置 AI 归档。

## 历史文档参考

### Code

- [Soure Code of labs](https://github.com/LearningOS/rCore-Tutorial-Code)

### Documents

- Concise Manual: [rCore-Tutorial-Guide](https://LearningOS.github.io/rCore-Tutorial-Guide/)

- Detail Book [rCore-Tutorial-Book-v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)

### OS API docs of rCore Tutorial Code

- [OS API docs of ch1](https://learningos.github.io/rCore-Tutorial-Code/ch1/os/index.html)
  AND [OS API docs of ch2](https://learningos.github.io/rCore-Tutorial-Code/ch2/os/index.html)
- [OS API docs of ch3](https://learningos.github.io/rCore-Tutorial-Code/ch3/os/index.html)
  AND [OS API docs of ch4](https://learningos.github.io/rCore-Tutorial-Code/ch4/os/index.html)
- [OS API docs of ch5](https://learningos.github.io/rCore-Tutorial-Code/ch5/os/index.html)
  AND [OS API docs of ch6](https://learningos.github.io/rCore-Tutorial-Code/ch6/os/index.html)
- [OS API docs of ch7](https://learningos.github.io/rCore-Tutorial-Code/ch7/os/index.html)
  AND [OS API docs of ch8](https://learningos.github.io/rCore-Tutorial-Code/ch8/os/index.html)
- [OS API docs of ch9](https://learningos.github.io/rCore-Tutorial-Code/ch9/os/index.html)

### Related Resources

- [Learning Resource](https://github.com/LearningOS/rust-based-os-comp2025/blob/main/relatedinfo.md)

### Build & Run

```bash
# setup build&run environment first
$ git clone https://github.com/LearningOS/rCore-Tutorial-Code.git
$ cd rCore-Tutorial-Code
$ git clone https://github.com/LearningOS/rCore-Tutorial-Test.git user
$ git checkout ch$ID
$ cd os
# run OS in ch$ID
$ make run
```

If you want to use docker to build and run, you can use the following command:
```bash
# After clone the `rCore-Tutorial-Test` repository to your local machine, you can use the following command to build and run:
$ make build_docker
$ make docker
```

If you experience network issues when accessing foreign resources such as GitHub in Docker, you can follow the following suggestions according to your stage:

- Docker pull:
  1. use proxy: https://docs.docker.com/reference/cli/docker/image/pull/#proxy-configuration

  2. use available domestic source (self-search)

- Docker build: use proxy https://docs.docker.com/engine/cli/proxy/#build-with-a-proxy-configuration

- Docker run: use proxy option, related operations are similar to `Docker build`, can refer to the relevant materials by yourself


Notice: $ID is from [1-9]

### Grading

```bash
# setup build&run environment first
$ git clone https://github.com/LearningOS/rCore-Tutorial-Code.git
$ cd rCore-Tutorial-Code
$ rm -rf ci-user
$ git clone https://github.com/LearningOS/rCore-Tutorial-Checker.git ci-user
$ git clone https://github.com/LearningOS/rCore-Tutorial-Test.git ci-user/user
$ git checkout ch$ID
# check&grade OS in ch$ID with more tests
$ cd ci-user && make test CHAPTER=$ID
```

Notice: $ID is from [3,4,5,6,8]
