# Flyline

[English](README.en.md) | **简体中文**

**Flyline** 是一个运行在 Bash 进程内的现代命令行编辑器，用 Rust 编写，直接替代 GNU readline。它以 Bash 可加载内建（loadable builtin）的形式加载进当前 shell，无需守护进程，提供流畅、丰富的交互体验。

> 本仓库是 fork 版本，在 [HalFrgrd/flyline](https://github.com/HalFrgrd/flyline) 的基础上加入了：中文界面（i18n）、宽字符（CJK/emoji）渲染修复、长按退格性能优化、右键菜单默认禁用、卸载崩溃修复等改进。

## 特性

- **智能自动建议**：输入时自动展开 Tab 补全候选（IntelliSense 风格）
- **Tab 补全增强**：描述列、动画描述、模糊过滤、基于路径/历史/别名的上下文补全
- **自动补全合成（flycomp）**：没有现成补全脚本时，从 `--help`/man 自动合成补全
- **富提示符**：PS1 / RPS1 / PS1_FILL / PS2，支持异步自定义组件与动画
- **模糊历史搜索**：快速回溯历史命令
- **鼠标支持**：点击移动光标、选择文本、悬停提示（右键菜单默认禁用，可开启）
- **语法高亮**：命令、参数、路径、环境变量、括号配对与彩虹括号
- **自动闭合**：引号、括号等自动配对，删除时智能清理
- **自定义光标**：动画、颜色、缓动效果，可用 `--no-custom-cursor` 关闭
- **Agent 模式**：用 AI 助手辅助编写命令
- **新手教程**：`flyline run-tutorial` 逐步上手
- **中文本地化**：根据 `FLYLINE_LANG` / `LC_ALL` / `LC_MESSAGES` / `LANG` 自动切换
- **宽字符渲染修复**：中文、emoji 在补全列表、右键菜单、教程等界面中不再错位或溢出

## 安装

### 从源码构建

需要 Rust 工具链（edition 2024）。

```bash
cargo build --release
```

加载到当前 Bash 会话：

```bash
enable -f target/release/libflyline.so flyline
```

卸载并恢复默认 readline：

```bash
enable -d flyline
```

建议把加载命令写入 `~/.bashrc`，并在新会话中运行 `flyline run-tutorial` 完成首次配置。

## 快速开始

```bash
# 第一次使用
flyline run-tutorial
```

## 常用配置

所有配置都是运行时命令，可以把喜欢的命令写进 `~/.bashrc` 持久化。

```bash
# 建议与补全
flyline suggestions --auto-suggest true
flyline suggestions --num-suggestion-rows 12

# 编辑器行为
flyline editor --show-inline-history true
flyline editor --auto-close-chars true

# 光标
flyline --no-custom-cursor                 # 不使用 flyline 自定义光标
flyline set-cursor --backend flyline
flyline set-cursor --style '#00ff00' --effect fade

# 鼠标
flyline --right-click-menu true            # 开启 flyline 右键菜单（默认禁用）
flyline mouse --mode smart                 # disabled / simple / smart

# 快捷键
flyline key bind Ctrl+g 'always=clearBuffer+submitOrNewline'
flyline key list

# Agent 模式
flyline set-agent-mode --command 'copilot --prompt' --system-prompt '...'

# 性能诊断
flyline perf start
flyline perf dump
flyline perf stop
```

> 说明：`--right-click-menu` 默认禁用。禁用时 flyline 不会请求终端鼠标捕获，右键事件由终端自己处理（例如终端的原生菜单/粘贴）；开启后 flyline 接管鼠标并显示自己的右键菜单。

## 提示符

Flyline 支持 Bash 原生 PS1 写法，也支持异步组件：

```bash
PS1='\u@\h:\w$ '
RPS1='\A'
```

更多组件（动画、命令输出、复制缓冲区、上次命令耗时等）请参考 `examples/widgets.sh`。

## 国际化

界面语言优先级：`FLYLINE_LANG` > `LC_ALL` > `LC_MESSAGES` > `LANG`。

```bash
export FLYLINE_LANG=zh_CN.UTF-8   # 强制中文
export LANG=zh_CN.UTF-8           # 跟随系统 locale
```

中文环境覆盖：教程、CLI 帮助、右键菜单、补全项描述、快捷键动作描述等。

## 鸣谢

- 上游项目 [HalFrgrd/flyline](https://github.com/HalFrgrd/flyline) 及其维护者，本 fork 基于其代码构建
- [ratatui](https://github.com/ratatui/ratatui) 终端 UI 框架
- [termina](https://github.com/HalFrgrd/termina)、[flycomp](https://github.com/HalFrgrd/flycomp)、[flash](https://github.com/HalFrgrd/flash)、[skim](https://github.com/lotabout/skim) 等生态项目
- 本 fork 的中文支持、宽字符修复、性能优化与稳定性修复，由 OpenAI Codex（AI 编程助手）协助开发调试

## 开发

```bash
cargo test --lib      # 单元测试（快速）
cargo fmt             # 代码格式化
cargo build --release
```

仓库结构、FFI 安全注意事项和调试指引见 [AGENTS.md](AGENTS.md)。

## 许可

本项目为多重许可：

- **源代码**：本仓库中的原始源代码以 [MIT License](LICENSE-MIT) 授权，可自由修改与复用。
- **预编译二进制与组合作品**：由于该内建会动态加载并链接 GNU Bash（GPLv3）的符号，分发的编译产物或组合作品受 [GNU General Public License v3](LICENSE-GPLv3) 约束。
