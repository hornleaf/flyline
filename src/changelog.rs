pub(crate) const CHANGELOG: &str = r#"# Changelog

## v1.5.0-fork
- **中文界面（i18n）**：教程、CLI 帮助、右键菜单、补全项描述与快捷键动作描述支持中文，根据 `FLYLINE_LANG` / `LC_ALL` / `LANG` 自动切换。
- **宽字符渲染修复**：中文、emoji 在补全列表、右键菜单、教程等界面中不再错位或溢出。
- **退格性能优化**：限制 Bash `type` 查询频率并跳过超长命令词，解决长按退格时 PATH 较慢（如 WSL）的卡顿。
- **右键菜单开关**：`--right-click-menu` 默认禁用；禁用时 flyline 不捕获鼠标，右键事件交给终端处理。
- **不使用自定义光标**：新增 `--no-custom-cursor` 选项。
- **卸载崩溃修复**：修复 `enable -d flyline` 在嵌套输入源下卸载导致 Bash 崩溃的问题。

## v1.5.0
- **Termina Backend**: Switched terminal rendering backend to `termina` for enhanced event handling and precise UI rendering.
- **Enhanced Mouse Selection & UX**: Added triple-click line selection, quad-click buffer selection, click-and-drag suggestion selection, and isolated scrolling movements.
- **Platform & Packaging Support**: Added Android/Termux installation support, a declarative NixOS module, and Homebrew installation documentation.
- **Binary Size & Build Optimization**: Reduced binary size by ~1.3MB by switching to `regex-lite` and improved Arch Linux LTO build options.
- **Agent & Subprocess Stability**: Fixed `SIGCHLD` signal handler reset behavior when spawning agent command substitutions to prevent process reaping errors (`ECHILD`).
- **Parsing & Completion Fixes**: Improved square bracket autoclosing, unterminated function acceptance, `autocd` directory path command recognition, quote space-suffix handling, and resolved `extglob` parsing issues.

## v1.4.0
- **Inline Viewport Smooth Height**: Viewport height pre-allocates to available space down to the bottom of the screen without scrolling up, eliminating viewport resize flicker when opening popups.
- **Third-Party Integration**: Enhanced support and terminal state synchronization for third-party tools (Atuin, FZF).
- **Customizable PS2**: Added support for customizable PS2 multi-line prompt rendering.
- **Packaging & Build Systems**: Added Nix flake packaging, Arch Linux build fixes, and `SOURCE_DATE_EPOCH` support for reproducible build timestamps.
- **Settings & Config**: Exposed Flycomp settings in Flyline and added options to disable easter eggs.
- **Bug Fixes & Stability**: Resolved PATH scan lock contention, zero-width terminal suggestion popup panics, and unterminated quote auto-newline insertion.

## v1.3.0
- **Leader Keys**: Added support for chorded keybinding sequences (e.g., `Ctrl+x` followed by `Ctrl+f`) via the new `setLeaderKey` and `unsetLeaderKey` actions and the `leaderKeyActive` context variable.
- **Leader Key Visual Feedback**: Introduced the `leader-mode` prompt widget to display visual indicators (like ` X `) in the prompt when the leader key state is active.
- **String Insertion Action**: `insertString(...)` action allows inserting arbitrary strings into the buffer.
- **Strict Modifier Matching**: Switched to strict modifier equality matching to prevent modifier-overlap conflicts when dispatching key actions.
- **Key List Autocomplete & Completion**: Added autocomplete support for listing keybindings for a specific key event (`flyline key list <key>`).

## v1.2.5
- **Global Allocator**: Integrated `mimalloc` to bypass Bash's non-thread-safe allocator and prevent heap corruption on multi-threaded allocations.
- **Nested Arithmetic Lexing**: Stateful lexing updates to correctly parse nested brackets/parentheses inside arithmetic `$(( ... ))` blocks.
- **Word Under Cursor breaks**: Updated word-under-cursor (WUC) detection to respect `:` and `=`, matching bash's standard `COMP_WORD_BREAK` behavior.
- **Kitty Cursor Support**: Added backend selection to keep the terminal emulator cursor visible on Kitty, preventing prompts when closing the window.

## v1.2.4
- **Safety Guards**: Fixed a Use-After-Free (UAF) issue, added safety guards, and enforced usage of the thread manager.
- **Mouse UX Improvements**: Corrected mouse event output formatting and resolved layout bugs, ensuring mouse event rows are always fully printed.
- **Robust WUC Handling**: Patched Word Under Cursor (WUC) edge cases and downgraded internal assertions to errors to prevent shell crashes.
- **AUR Package**: Documented and referenced the official Arch Linux User Repository (AUR) package.
- **Cleanups**: Removed the legacy `get_current_readline_prompt` hook dependency to streamline FFI interactions.

## v1.2.3
- **Thread Safety**: Added `BASH_LOCK` to prevent concurrency crashes when accessing Bash FFI from background threads.
- **Log Forwarding**: Pipes tab-completion child logs back to the parent to prevent double-logging and preserve trails.
- **Fuzzy Mode**: Added `flyline suggestions set-fuzzy-mode` (`all`, `none`, `folder-prefixes`) for folder prefix matching.

## v1.2.2
- **Changelog Command**: Added `flyline changelog` command to display user-facing changelogs directly in the pager.
- **Upgrade Assistant**: Added `flyline upgrade` command which pre-fills the prompt line with the curl installer command.
- **Installer improvements**: Streamlined `install.sh` to run non-interactively, resolving target folders automatically.

## v1.2.1
- **Declarative Mouse Actions**: Re-architected mouse event processing into a declarative, context-aware routing system.
- **Tab Completion Latency**: Reduced visual flashing during tab completion redraws and optimized filtering latency for large lists.
- **Offline Installer**: Updated `install.sh` to bypass GitHub API rate limits by resolving release redirect headers.
- **Wider Platform Support**: Added release builds for FreeBSD, ARMv7, 32-bit x86, RISC-V 64, and PowerPC 64 LE.
- **OSC 52 Paste**: Replaced custom OSC 52 querying with crossterm's native RequestClipboardContents.

## v1.2.0
- **Transient Prompts**: Added support for transient prompts, reducing terminal noise by condensing past prompts upon execution.
- **History Management**: Introduced separate history managers for cancelled commands and agent prompts.
- **Non-blocking Completion**: Improved tab-completion responsiveness by spawning completion generation in a dedicated process.
- **Scroll & Right-Click UX**: Enhanced right-click context menu and continuous proportional scrollbar dragging.

## v1.1.0
- **Fuzzy Sorting**: Introduced suggestion sorting algorithms (mtime, alphabetical) and CLI configuration options.
- **Improved Parsing**: Enhanced flycomp parsing for cargo, git --help, and flag values ending in `=`.
- **Fuzzy Matching**: Tightened fuzzy suggestion matching and fixed scrollbar positions.

## v1.0.0
- **Stable Line Editor**: First major release of the Rust-based GNU readline replacement builtin for Bash.
- **Mouse Selection**: Support for cursor placement and visual drag-selections using mouse.
- **Auto-Closing pairs**: Automatic insertion of closing quotes, brackets, and parentheses.
- **Interactive Tutorial**: Added an in-terminal tutorial to guide users through keyboard and mouse controls.
"#;
