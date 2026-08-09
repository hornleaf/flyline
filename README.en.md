# Flyline

**English** | [简体中文](README.md)

**Flyline** is a modern command-line editor that runs inside the Bash process. Written in Rust, it replaces GNU readline and loads directly into the current shell as a loadable builtin — no daemon required.

> This repository is a fork of [HalFrgrd/flyline](https://github.com/HalFrgrd/flyline) with additional improvements: Chinese localization (i18n), wide-character (CJK/emoji) rendering fixes, backspace-hold performance optimizations, a right-click menu that is disabled by default, and unload crash fixes.

## Features

- **Intellisense-style auto suggestions**: tab-completion candidates expand automatically as you type
- **Enhanced tab completion**: description columns, animated descriptions, fuzzy filtering, and context-aware completion from paths, history, and aliases
- **Automatic completion synthesis (flycomp)**: generates completion scripts from `--help`/man pages when no compspec exists
- **Rich prompts**: PS1 / RPS1 / PS1_FILL / PS2 with async custom widgets and animations
- **Fuzzy history search**: quickly recall past commands
- **Mouse support**: click to move the cursor, select text, hover tooltips (right-click menu is disabled by default and can be enabled)
- **Syntax highlighting**: commands, arguments, paths, environment variables, paired and rainbow brackets
- **Auto-closing pairs**: quotes, brackets, and other delimiters with smart deletion
- **Custom cursor**: animations, colors, easing — disable with `--no-custom-cursor`
- **Agent mode**: use an AI assistant to help write commands
- **Interactive tutorial**: `flyline run-tutorial` walks you through the basics
- **Chinese localization**: automatically selected via `FLYLINE_LANG` / `LC_ALL` / `LC_MESSAGES` / `LANG`
- **Wide-character rendering fixes**: Chinese and emoji no longer misalign or overflow in completion lists, menus, and tutorials

## Installation

### Build from source

Requires a Rust toolchain (edition 2024).

```bash
cargo build --release
```

Load into the current Bash session:

```bash
enable -f target/release/libflyline.so flyline
```

Unload and restore default readline:

```bash
enable -d flyline
```

Add the load command to your `~/.bashrc`, then run `flyline run-tutorial` in a new session to get started.

## Quick start

```bash
# First time use
flyline run-tutorial
```

## Common configuration

All configuration is applied at runtime; add the commands you like to `~/.bashrc` to persist them.

```bash
# Suggestions & completion
flyline suggestions --auto-suggest true
flyline suggestions --num-suggestion-rows 12

# Editor behavior
flyline editor --show-inline-history true
flyline editor --auto-close-chars true

# Cursor
flyline --no-custom-cursor                 # use the terminal cursor
flyline set-cursor --backend flyline
flyline set-cursor --style '#00ff00' --effect fade

# Mouse
flyline --right-click-menu true            # enable flyline's right-click menu (disabled by default)
flyline mouse --mode smart                 # disabled / simple / smart

# Keybindings
flyline key bind Ctrl+g 'always=clearBuffer+submitOrNewline'
flyline key list

# Agent mode
flyline set-agent-mode --command 'copilot --prompt' --system-prompt '...'

# Performance diagnostics
flyline perf start
flyline perf dump
flyline perf stop
```

> Note: `--right-click-menu` is disabled by default. When disabled, flyline does not request terminal mouse capture, so right-click events stay with the terminal (its native menu/paste). When enabled, flyline takes over the mouse and shows its own right-click menu.

## Prompts

Flyline supports native Bash PS1 syntax as well as async widgets:

```bash
PS1='\u@\h:\w$ '
RPS1='\A'
```

For more widgets (animations, command output, copy buffer, last-command duration, etc.), see `examples/widgets.sh`.

## Internationalization

Language priority: `FLYLINE_LANG` > `LC_ALL` > `LC_MESSAGES` > `LANG`.

```bash
export FLYLINE_LANG=zh_CN.UTF-8   # force Chinese
export LANG=zh_CN.UTF-8           # follow the system locale
```

Chinese localization covers the tutorial, CLI help, right-click menu, completion descriptions, and keybinding action descriptions.

## Acknowledgements

- The upstream [HalFrgrd/flyline](https://github.com/HalFrgrd/flyline) project and its maintainers, on which this fork is based
- The [ratatui](https://github.com/ratatui/ratatui) terminal UI framework
- Ecosystem projects such as [termina](https://github.com/HalFrgrd/termina), [flycomp](https://github.com/HalFrgrd/flycomp), [flash](https://github.com/HalFrgrd/flash), and [skim](https://github.com/lotabout/skim)
- The Chinese localization, wide-character fixes, performance work, and stability fixes in this fork were developed and debugged with assistance from OpenAI Codex (AI coding assistant)

## Development

```bash
cargo test --lib      # fast unit tests
cargo fmt             # format the codebase
cargo build --release
```

For repository structure, FFI safety notes, and debugging guidance, see [AGENTS.md](AGENTS.md).

## Licensing

This project is multi-licensed:

- **Source code**: the original source code in this repository is licensed under the [MIT License](LICENSE-MIT).
- **Precompiled binaries & combined works**: because this builtin dynamically loads and links against symbols from GNU Bash (GPLv3), distributed compiled binaries or combined works are governed by the [GNU General Public License v3](LICENSE-GPLv3).
