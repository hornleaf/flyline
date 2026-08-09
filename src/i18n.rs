//! Lightweight localization support (Simplified Chinese).
//!
//! The active language is detected from `FLYLINE_LANG` (explicit override)
//! followed by the standard `LC_ALL` / `LC_MESSAGES` / `LANG` environment
//! variables.  User-facing strings are wrapped in [`crate::t!`]; a key with no
//! translation falls back to the English source text.
//!
//! CLI help text is localized at runtime by rewriting the clap [`Command`]
//! tree with translated help strings (see [`localize_clap_command`]).

use std::sync::OnceLock;

use clap::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Language {
    En,
    Zh,
}

static CURRENT_LANGUAGE: OnceLock<Language> = OnceLock::new();

pub(crate) fn current_language() -> Language {
    *CURRENT_LANGUAGE.get_or_init(detect_language)
}

/// Localize an arbitrary help/description string in the active language,
/// falling back to the original text when no translation exists.
pub(crate) fn localize_help_text(text: &str) -> String {
    match current_language() {
        Language::En => text.to_string(),
        Language::Zh => zh_help_translation(text).unwrap_or(text).to_string(),
    }
}

/// Returns the translation of `key` in the active language, or `key` itself
/// when no translation exists.
pub(crate) fn translate(key: &'static str) -> &'static str {
    match current_language() {
        Language::En => key,
        Language::Zh => zh_translation(key).unwrap_or(key),
    }
}

/// Translates a format template (`{}` / `{:?}` placeholders) and substitutes
/// the pre-formatted arguments in order.  Used where the format string cannot
/// be a literal (e.g. `eprintln!`), so arguments must be formatted by callers
/// with `to_string()` / `format!("{:?}", x)`.
pub(crate) fn translate_fmt(template: &'static str, args: &[String]) -> String {
    let translated = translate(template);
    let mut out = String::with_capacity(translated.len());
    let mut rest = translated;
    let mut args_iter = args.iter();
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        if let Some(end_rel) = rest[start..].find('}') {
            let end = start + end_rel;
            if let Some(arg) = args_iter.next() {
                out.push_str(arg);
            } else {
                out.push_str(&rest[start..=end]);
            }
            rest = &rest[end + 1..];
        } else {
            out.push_str(&rest[start..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// Translate `key` to Simplified Chinese, or `None` when untranslated.
fn zh_translation(key: &str) -> Option<&'static str> {
    Some(match key {
        // Welcome screen
        "Press Enter to start the tutorial" => "按 Enter 开始教程",

        // Tutorial: how to use this tutorial
        "How to use this tutorial" => "如何使用本教程",
        "• Click the prev and next buttons to navigate." => {
            "• 点击“上一个”和“下一个”按钮进行导航。"
        }
        "• Press " => "• 按 ",
        " with an empty command buffer to move to the next tutorial screen." => {
            " 并在命令缓冲区为空时进入下一个教程页面。"
        }
        "• Click on underlined text to copy it to your clipboard and command buffer: " => {
            "• 点击带下划线的文本可将其复制到剪贴板和命令缓冲区："
        }
        "• Exit the tutorial at any time with `flyline run-tutorial false`." => {
            "• 随时使用 `flyline run-tutorial false` 退出教程。"
        }
        "• Remember to append settings to your `~/.bashrc` so they persist!" => {
            "• 记得把设置追加到你的 `~/.bashrc`，以便持久生效！"
        }

        // Tutorial: recommended settings
        "Recommended Settings" => "推荐设置",
        "Flyline will detect your terminal and suggest optimal settings for the best experience:" => {
            "Flyline 会检测你的终端，并推荐最佳体验的优化设置："
        }
        "You are running in VS Code. For the best experience, set these in settings.json (try ctrl+clicking the links):" => {
            "你正在 VS Code 中运行。为获得最佳体验，请在 settings.json 中设置以下选项（可尝试 Ctrl+点击链接）："
        }
        "You are running in Ghostty. Consider setting this configuration to prevent mouse click conflicts:" => {
            "你正在 Ghostty 中运行。建议设置以下配置，避免鼠标点击冲突："
        }
        "✅ Your terminal supports the Kitty extended keyboard protocol." => {
            "✅ 你的终端支持 Kitty 扩展键盘协议。"
        }
        "⚠ Your terminal may not support the Kitty extended keyboard protocol." => {
            "⚠ 你的终端可能不支持 Kitty 扩展键盘协议。"
        }
        "  Consider using a terminal emulator that does (kitty, ghostty, wezterm, foot, rio)." => {
            "  请考虑使用支持该协议的终端模拟器（kitty、ghostty、wezterm、foot、rio）。"
        }
        "  This enables better key disambiguation for flyline." => {
            "  这能让 flyline 更好地区分按键。"
        }
        "💡 We detected that you use Zsh. Consider loading your Zsh history into flyline:" => {
            "💡 检测到你正在使用 Zsh。建议将 Zsh 历史记录加载到 flyline："
        }
        "💡 How about showing the time in your right prompt:" => {
            "💡 不妨在右侧提示符中显示时间："
        }

        // Tutorial: mouse capture
        "Mouse Capture" => "鼠标捕获",
        "Flyline needs to capture mouse events so that you can click to move your cursor, select suggestions, and hover for tooltips." => {
            "Flyline 需要捕获鼠标事件，以便你点击移动光标、选择建议，以及悬停查看提示。"
        }
        "Disable mouse capture: click above the viewport or scroll." => {
            "禁用鼠标捕获：点击视口上方或滚动。"
        }
        "Toggle with " => "使用 ",
        "Typing enables mouse capture." => "开始输入将重新启用鼠标捕获。",
        "Switch mouse interaction modes with `flyline mouse --mode smart/simple/disabled`." => {
            "使用 `flyline mouse --mode smart/simple/disabled` 切换鼠标交互模式。"
        }
        "💡 Consider displaying the mouse capture mode in your right prompt:" => {
            "💡 考虑在右侧提示符中显示鼠标捕获模式："
        }

        // Tutorial: text selection & clipboard
        "Text Selection & Clipboard" => "文本选择与剪贴板",
        "• Select text by dragging the mouse or using Shift + Arrow keys." => {
            "• 拖拽鼠标或使用 Shift + 方向键选择文本。"
        }
        "• Right-click to open the context menu to Copy, Cut, or Paste." => {
            "• 右键打开上下文菜单进行复制、剪切或粘贴。"
        }
        "• You can also right-click a history entry or a prompt folder to copy it directly." => {
            "• 也可以右键历史记录条目或提示符目录直接复制。"
        }
        "• Ctrl+X, Ctrl+C, and Ctrl+V work as expected." => {
            "• Ctrl+X、Ctrl+C 和 Ctrl+V 均按预期工作。"
        }
        "• Ctrl+C will copy when text is selected and if not, it will cancel your command." => {
            "• 选中文本时 Ctrl+C 会复制；未选中时则取消当前命令。"
        }

        // Tutorial: auto suggestions
        "Auto Suggestions" => "自动建议",
        "As you type, flyline shows Intellisense style auto-suggestions based on Bash tab completions." => {
            "输入时，flyline 会基于 Bash 的 Tab 补全显示类似 Intellisense 的自动建议。"
        }
        "Try typing `grep --` and watch suggestions appear." => {
            "试试输入 `grep --`，观察建议出现。"
        }
        "You can disable these auto-suggestions by running:" => {
            "你可以通过运行以下命令禁用这些自动建议："
        }

        // Tutorial: fuzzy history search
        "Fuzzy History Search" => "模糊历史搜索",
        " to open fuzzy history search." => " 打开模糊历史搜索。",
        "Type to filter, use " => "输入以过滤，使用 ",
        "arrow keys" => "方向键",
        " to browse results." => " 浏览结果。",
        " to accept the selected command for editing." => " 接受选中的命令以进行编辑。",
        " to cancel." => " 取消。",

        // Tutorial: keybindings
        "Keybindings" => "按键绑定",
        " to see all current keybindings." => " 查看当前所有按键绑定。",
        "Common custom keybindings:" => "常用自定义按键绑定：",
        "• Accept and immediately run the selected fuzzy history entry (instead of accepting for editing):" => {
            "• 接受并立即运行选中的模糊历史条目（而不是先接受编辑）："
        }
        "• Temporarily dismiss an inline history suggestion with " => "• 使用 ",
        "• Accept an inline history suggestion with " => "• 使用 ",

        // Tutorial: tab suggestions
        "Fuzzy Completions" => "模糊补全",
        "Type " => "输入 ",
        " and press " => " 并按 ",
        " to trigger completions. If nothing comes up, first set normal Bash completions (" => {
            " 触发补全。如果没有出现补全，请先安装 Bash 常规补全（"
        }
        "Type to filter suggestions, use " => "输入过滤建议，使用 ",
        " or your mouse to navigate." => " 或鼠标进行导航。",
        " or click a suggestion to accept it." => " 或点击建议以接受。",

        // Tutorial: theme colours
        "Setting Theme Colours" => "设置主题颜色",
        "Customise your colour theme with the `flyline set-style` command." => {
            "使用 `flyline set-style` 命令自定义颜色主题。"
        }
        "Examples:" => "示例：",
        " (if your terminal background is dark)" => "（如果你的终端背景为深色）",
        " (if your terminal background is light)" => "（如果你的终端背景为浅色）",
        "Run " => "运行 ",
        " to see all options." => " 查看所有选项。",

        // Tutorial: cursor style & effects
        "Cursor Style & Effects" => "光标样式与效果",
        "⚠ Warning: You are running Kitty. The custom `flyline` cursor backend hides the terminal's native cursor, which stops Kitty from detecting prompt states and prompts you on exit. It is highly recommended to use the `terminal` backend instead." => {
            "⚠ 警告：你正在运行 Kitty。自定义的 `flyline` 光标后端会隐藏终端原生光标，导致 Kitty 无法检测提示符状态并在退出时提示。强烈建议改用 `terminal` 后端。"
        }
        "Use " => "使用 ",
        " to control how the cursor looks and animates." => " 控制光标的外观和动画。",
        "Style and effect options require the `flyline` cursor backend. The `terminal` backend leaves cursor rendering to your terminal emulator." => {
            "样式和效果选项需要 `flyline` 光标后端。`terminal` 后端将光标渲染交给终端模拟器。"
        }
        " (your terminal emulator will render the cursor)" => "（由终端模拟器渲染光标）",
        " (invert the character under the cursor)" => "（反转光标下的字符）",
        " (custom foreground, background, and style)" => "（自定义前景色、背景色和样式）",
        " (faster blinking cursor)" => "（更快闪烁的光标）",
        " (RGB fade effect with smooth easing and bouncing interpolation when the cursor moves)" => {
            "（光标移动时带有平滑缓动和弹跳插值的 RGB 渐隐效果）"
        }
        "Try tab completing " => "试试对 ",
        " for an example of flyline's dynamic tab completion descriptions!" => {
            " 进行 Tab 补全，体验 flyline 的动态补全描述！"
        }

        // Tutorial: auto-closing
        "Auto-Closing Quotes & Brackets" => "引号与括号自动闭合",
        "Flyline automatically inserts closing characters when you type an opening one." => {
            "当你输入左括号或引号时，Flyline 会自动插入对应的闭合字符。"
        }
        "Try typing `echo \"$(` and watch Flyline insert the closing `)\"` for you." => {
            "试试输入 `echo \"$(`，观察 Flyline 自动补上闭合的 `)\"`。"
        }
        "This works for parentheses (), square brackets [], curly braces {}, and quotes \" \"." => {
            "适用于圆括号 ()、方括号 []、花括号 {} 和引号 \" \"。"
        }
        "Toggle this feature with " => "使用 ",

        // Tutorial: fine-grained deletion
        "Fine-Grained Deletion" => "精细删除",
        " deletes one whitespace-delimited word to the left." => " 删除左侧一个以空白分隔的单词。",
        " deletes one chunk to the left using finer punctuation or path-segment boundaries." => {
            " 按更细的标点或路径段边界删除左侧一个片段。"
        }
        " and " => " 和 ",
        " work similarly." => " 操作类似。",
        "Try it out on this example command:" => "在以下示例命令上试试：",

        // Tutorial: agent mode
        "Agent Mode" => "AI 助手模式",
        "Flyline can interface with your AI agent to help you write commands." => {
            "Flyline 可以与你的 AI 助手协作，帮助你编写命令。"
        }
        "Try activating agent mode and get help setting it up:" => {
            "试试激活 AI 助手模式，获取设置帮助："
        }
        "` and press " => "` 并按 ",
        "When setting it up, you can specify a `--trigger-prefix`. If the buffer starts with this prefix, flyline will activate agent mode when you press " => {
            "设置时，可以指定 `--trigger-prefix`。如果缓冲区以此前缀开头，按下 "
        }

        // Tutorial: end
        "You've reached the end of the tutorial!" => "教程已结束！",
        "Feel free to explore and experiment with flyline's features." => {
            "欢迎继续探索和体验 flyline 的功能。"
        }
        "For more information, check out " => "更多信息，请查看 ",
        " and https://github.com/HalFrgrd/flyline." => {
            " 以及 https://github.com/HalFrgrd/flyline。"
        }

        // UI: mouse / tutorial hints
        "Press Escape to re-enable mouse mode." => "按 Escape 重新启用鼠标模式。",

        // UI: flycomp sandbox prompt
        "  Proceed? " => "  是否继续？ ",
        " [Yes] " => " [是] ",
        " [No] " => " [否] ",
        " [No, don't ask again] " => " [否，不再询问] ",

        // CLI usage errors
        "flyline set-agent-mode: --command must not be empty" => {
            "flyline set-agent-mode：--command 不能为空"
        }
        "flyline create-prompt-widget animation: --fps must be greater than 0 (got {}); animation '{}' not registered" => {
            "flyline create-prompt-widget animation：--fps 必须大于 0（当前为 {}）；动画 '{}' 未注册"
        }
        "flyline create-prompt-widget custom: --command must not be empty" => {
            "flyline create-prompt-widget custom：--command 不能为空"
        }
        "flyline create-prompt-widget custom: --block timeout must be non-negative (got {})" => {
            "flyline create-prompt-widget custom：--block 超时时间必须为非负数（当前为 {}）"
        }
        "flyline create-prompt-widget custom: --placeholder must be a number or 'prev', got {:?}" => {
            "flyline create-prompt-widget custom：--placeholder 必须是数字或 'prev'，当前为 {:?}"
        }
        "flyline set-style: argument must be NAME=STYLE, got {:?}" => {
            "flyline set-style：参数必须是 NAME=STYLE 格式，当前为 {:?}"
        }
        "flyline set-style: unknown style name {:?}. Run 'flyline set-style --help' for valid names." => {
            "flyline set-style：未知样式名称 {:?}。运行 'flyline set-style --help' 查看有效名称。"
        }
        "flyline set-style: invalid style for {:?}: {}" => "flyline set-style：{:?} 的样式无效：{}",
        "flyline key bind: {}" => "flyline key bind：{}",
        "flyline key remap: failed to parse remap '{}' -> '{}': {}" => {
            "flyline key remap：无法解析重映射 '{}' -> '{}'：{}"
        }
        "flyline suggestions: --num-suggestion-rows must be greater than 0" => {
            "flyline suggestions：--num-suggestion-rows 必须大于 0"
        }
        "flyline time: invalid Chrono format string: {:?}" => {
            "flyline time：无效的 Chrono 格式字符串：{:?}"
        }
        "flyline set-cursor: --style, --effect, --effect-speed, and --effect-easing require --backend flyline" => {
            "flyline set-cursor：--style、--effect、--effect-speed 和 --effect-easing 需要 --backend flyline"
        }
        "flyline set-cursor: --interpolate must be a positive number or 'none' (got {:?})" => {
            "flyline set-cursor：--interpolate 必须为正数或 'none'（当前为 {:?}）"
        }
        "flyline set-cursor: --style requires --backend flyline" => {
            "flyline set-cursor：--style 需要 --backend flyline"
        }
        "flyline set-cursor: invalid --style {:?}: {}" => {
            "flyline set-cursor：无效的 --style {:?}：{}"
        }
        "flyline set-cursor: --effect requires --backend flyline" => {
            "flyline set-cursor：--effect 需要 --backend flyline"
        }
        "flyline set-cursor: --effect fade requires a custom style with an RGB background color (e.g. '#ff0000')" => {
            "flyline set-cursor：--effect fade 需要带有 RGB 背景色的自定义样式（例如 '#ff0000'）"
        }
        "flyline set-cursor: --effect-speed requires --backend flyline" => {
            "flyline set-cursor：--effect-speed 需要 --backend flyline"
        }
        "flyline set-cursor: --effect-speed must be positive (got {})" => {
            "flyline set-cursor：--effect-speed 必须为正数（当前为 {}）"
        }
        "flyline set-cursor: --effect-easing requires --backend flyline" => {
            "flyline set-cursor：--effect-easing 需要 --backend flyline"
        }

        // Other user-facing warnings
        "Warning: could not parse key sequence '{}'" => "警告：无法解析按键序列 '{}'",

        // Right-click context menu
        "⎘ Copy (selection)" => "⎘ 复制（选中内容）",
        "⎘ Copy (buffer)" => "⎘ 复制（缓冲区）",
        "⎘ Copy (history entry)" => "⎘ 复制（历史条目）",
        "⎘ Copy (cwd)" => "⎘ 复制（当前目录）",
        "⎘ Copy (suggestion)" => "⎘ 复制（建议）",
        "⎘ Copy (AI result)" => "⎘ 复制（AI 结果）",
        "⎘ Copy (clipboard)" => "⎘ 复制（剪贴板）",
        "⎘ Copy" => "⎘ 复制",
        "✂ Cut (selection)" => "✂ 剪切（选中内容）",
        "✂ Cut (buffer)" => "✂ 剪切（缓冲区）",
        "⎗ Paste" => "⎗ 粘贴",
        "↶ Undo" => "↶ 撤销",
        "↷ Redo" => "↷ 重做",
        "Run Tutorial" => "运行新手教程",
        "Toggle mouse capture" => "切换鼠标捕获",
        "with Escape." => "使用 Escape。",

        // Command type descriptions in completions
        "unknown" => "未知命令",
        "alias: {}" => "别名：{}",
        "keyword: {}" => "关键字：{}",
        "builtin: {}" => "内建：{}",
        "function {}:{}" => "函数 {}:{}",
        "function {}" => "函数 {}",
        "function :{}" => "函数 :{}",
        "function" => "函数",

        _ => return None,
    })
}

/// Translate a CLI help/about string to Simplified Chinese, or `None` when no
/// translation exists (the English text is kept).  Clap strips the trailing
/// period from non-verbatim doc comments, so we also try the key without it.
fn zh_help_translation(key: &str) -> Option<&'static str> {
    if let Some(translated) = zh_help_translation_exact(key) {
        return Some(translated);
    }
    if let Some(stripped) = key.strip_suffix('.') {
        return zh_help_translation_exact(stripped);
    }
    None
}

fn zh_help_translation_exact(key: &str) -> Option<&'static str> {
    Some(match key {
        // Top-level options
        "Show version information" => "显示版本信息",
        "Load Zsh history in addition to Bash history. Optionally specify a PATH to the Zsh history file" => {
            "额外加载 Zsh 历史记录（除 Bash 历史外）。可选地指定 Zsh 历史文件的路径"
        }
        "Show animations" => "显示动画",
        "Run matrix animation in the terminal background. Use `on` to always show it, `off` to disable it, or an integer number of seconds to show it after that many seconds of inactivity (no keypress or mouse event). Defaults to `off`; passing the flag without a value is equivalent to `on`" => {
            "在终端后台运行矩阵动画。使用 `on` 始终显示，`off` 关闭，或指定一个整数秒数，在空闲（无按键或鼠标事件）那么多秒后显示。默认为 `off`；不带值地传递该参数等价于 `on`"
        }
        "Render frame rate in frames per second (1–120, default 24)" => {
            "以每秒帧数设置渲染帧率（1–120，默认 24）"
        }
        "Mouse capture mode (disabled, simple, smart). Default is smart" => {
            "鼠标捕获模式（disabled、simple、smart）。默认为 smart"
        }
        "Send shell integration escape codes (OSC 133 / OSC 633): none, only-prompt-pos, or full" => {
            "发送 shell 集成转义码（OSC 133 / OSC 633）：none、only-prompt-pos 或 full"
        }
        "Whether to request the use of extended (kitty-protocol) keyboard codes during startup. Enabled by default; pass `--enable-extended-key-codes false` to disable it on terminals that misbehave when the request is sent" => {
            "是否在启动时请求使用扩展（kitty 协议）键盘码。默认启用；在发送请求后行为异常的终端上，可传递 `--enable-extended-key-codes false` 关闭"
        }
        "Whether easter eggs (such as animated command words like `python`) are enabled. Enabled by default; pass `--enable-easter-eggs false` to disable" => {
            "是否启用彩蛋（例如 `python` 等命令词的动画效果）。默认启用；传递 `--enable-easter-eggs false` 可关闭"
        }
        "Do not render a custom flyline cursor; leave cursor rendering entirely to the terminal emulator. Equivalent to `flyline set-cursor --backend terminal`" => {
            "不渲染 flyline 自定义光标，将光标渲染完全交给终端模拟器。等价于 `flyline set-cursor --backend terminal`"
        }

        // Subcommand about text
        "Copy version information to clipboard" => "将版本信息复制到剪贴板",
        "Print a timestamp" => "打印时间戳",
        "Configure AI agent mode" => "配置 AI 助手模式",
        "Create a custom prompt widget" => "创建自定义提示符组件",
        "Configure the colour palette" => "配置颜色主题",
        "Configure the cursor appearance and animation" => "配置光标外观与动画",
        "Manage keybindings" => "管理按键绑定",
        "List all keybindings from lowest to highest priority" => {
            "按优先级从低到高列出所有按键绑定"
        }
        "Control mouse capture" => "控制鼠标捕获",
        "Control the tutorial" => "控制新手教程",
        "Configure suggestions" => "配置建议",
        "Configure editor features" => "配置编辑器功能",
        "Configure agent mode" => "配置 AI 助手模式",
        "Logging commands: dump, configure level, or stream logs" => {
            "日志命令：dump、configure level 或 stream logs"
        }
        "Run the interactive tutorial for first-time users" => "为首次用户运行交互式新手教程",
        "Configure the inline editor" => "配置内联编辑器",
        "Configure suggestion behavior" => "配置建议行为",
        "Configure mouse options and debugging" => "配置鼠标选项与调试",
        "Performance profiling commands: start, stop, or dump stats" => {
            "性能分析命令：start、stop 或 dump 统计信息"
        }
        "Display the changelog of user-facing changes" => "显示面向用户的变更日志",
        "Display instructions to upgrade flyline" => "显示升级 flyline 的说明",
        "Bind a key sequence to an action, optionally guarded by a context expression" => {
            "将按键序列绑定到动作，可选地通过上下文表达式限定条件"
        }
        "Remap a key or modifier to another key or modifier" => {
            "将按键或修饰键重映射为另一个按键或修饰键"
        }
        "Create a custom prompt animation that cycles through frames" => {
            "创建循环播放帧的自定义提示符动画"
        }
        "Run a shell command and display its output in the prompt" => {
            "运行 shell 命令并在提示符中显示其输出"
        }
        "Show clickable text that copies the current command buffer to the clipboard" => {
            "显示可点击的文本，将当前命令缓冲区复制到剪贴板"
        }
        "Show different text depending on whether mouse capture is enabled" => {
            "根据鼠标捕获是否启用显示不同的文本"
        }
        "Show different text depending on whether the leader key is active" => {
            "根据 leader 键是否激活显示不同的文本"
        }
        "Show how long ago the flyline app last closed in the prompt" => {
            "在提示符中显示 flyline 应用上次关闭的时间"
        }
        "Configure flycomp settings" => "配置 flycomp 设置",
        "Set fuzzy matching mode (all, none, no folders)" => {
            "设置模糊匹配模式（all、none、no folders）"
        }
        "Set the logging level" => "设置日志级别",
        "Copy in-memory log entries to the clipboard" => "将内存中的日志条目复制到剪贴板",
        "Dump all in-memory log entries to stdout" => "将内存中的所有日志条目转储到 stdout",
        "Stream logs to a file path or to the terminal" => "将日志流式输出到文件路径或终端",
        "Start recording performance metrics" => "开始记录性能指标",
        "Stop recording performance metrics" => "停止记录性能指标",
        "Dump aggregated performance metrics to stdout" => "将聚合的性能指标转储到 stdout",

        // Subcommand arguments
        "Format string passed to Chrono's `strftime` formatter. When omitted, prints nanoseconds since the Unix epoch" => {
            "传给 Chrono `strftime` 格式化器的格式字符串。省略时打印自 Unix 纪元以来的纳秒数"
        }
        "Optional system prompt prepended to the buffer. The subprocess receives \"<system-prompt>\\n<buffer>\" as its final argument" => {
            "可选：添加到缓冲区之前的系统提示词。子进程将收到 \"<system-prompt>\\n<buffer>\" 作为其最终参数"
        }
        "Optional trigger prefix. When set, pressing Enter with a buffer that starts with this prefix activates agent mode (the prefix is stripped)" => {
            "可选：触发前缀。设置后，当缓冲区以此前缀开头并按 Enter 时，将激活 AI 助手模式（前缀会被移除）"
        }
        "Command string to invoke; include any flags in the same string, e.g. --command 'copilot --reasoning-effort low --prompt'. The current buffer is appended as the final argument when Alt+Enter is pressed" => {
            "要调用的命令字符串；将任何参数包含在同一字符串中，例如 --command 'copilot --reasoning-effort low --prompt'。按 Alt+Enter 时，当前缓冲区会作为最终参数追加"
        }
        "Command string to run; include any flags in the same string, e.g. --command './widget.sh --someflag'" => {
            "要运行的命令字符串；将任何参数包含在同一字符串中，例如 --command './widget.sh --someflag'"
        }
        "Apply a built-in colour preset for dark or light terminals" => {
            "应用适用于深色或浅色终端的内置颜色预设"
        }
        "One or more palette style assignments as NAME=STYLE. NAME is the kebab-case style slot name; STYLE is a rich-style string" => {
            "一个或多个调色板样式赋值，格式为 NAME=STYLE。NAME 是短横线形式的样式槽名称；STYLE 是富样式字符串"
        }
        "Cursor rendering backend.  `flyline` renders a custom cursor (the default); `terminal` defers to the terminal emulator" => {
            "光标渲染后端。`flyline` 渲染自定义光标（默认）；`terminal` 交给终端模拟器处理"
        }
        "Interpolation speed (1/second), or `none` to disable interpolation.  Default is `16`" => {
            "插值速度（1/秒），或 `none` 禁用插值。默认为 `16`"
        }
        "Easing function for position interpolation.  Default is `linear`" => {
            "位置插值的缓动函数。默认为 `linear`"
        }
        "Cursor style.  A single colour (e.g. `red`) is the cursor background. `\"pink on white\"` sets foreground and background.  `\"reverse\"` inverts the cell colours.  Default is a white block modulated by the effect" => {
            "光标样式。单个颜色（例如 `red`）为光标背景。`\"pink on white\"` 设置前景色和背景色。`\"reverse\"` 反转单元格颜色。默认为受效果调制的白色块"
        }
        "Visual effect applied to the cursor: `fade`, `blink`, or `none`" => {
            "应用于光标的效果：`fade`、`blink` 或 `none`"
        }
        "Speed multiplier for the cursor effect (default is `1.0`)" => {
            "光标效果的速度倍率（默认为 `1.0`）"
        }
        "Easing function for the cursor effect intensity.  Default is `linear`" => {
            "光标效果强度的缓动函数。默认为 `linear`"
        }
        "Key sequence to bind (e.g. \"Ctrl+Enter\", \"Alt+Left\")" => {
            "要绑定的按键序列（例如 \"Ctrl+Enter\"、\"Alt+Left\"）"
        }
        "Context expression and action in the form `<contextExpr>=<actionName>` (e.g. \"always=submitOrNewline\")" => {
            "上下文表达式和动作，格式为 `<contextExpr>=<actionName>`（例如 \"always=submitOrNewline\"）"
        }
        "Optional key sequence to filter by (e.g. \"Tab\", \"Ctrl+r\")" => {
            "可选：用于过滤的按键序列（例如 \"Tab\"、\"Ctrl+r\"）"
        }
        "The key or modifier to remap from (e.g. \"tab\", \"alt\")" => {
            "要重映射的按键或修饰键（例如 \"tab\"、\"alt\"）"
        }
        "The key or modifier to remap to (e.g. \"z\", \"ctrl\")" => {
            "重映射为的按键或修饰键（例如 \"z\"、\"ctrl\"）"
        }
        "Show the last key event and dispatched action above the prompt" => {
            "在提示符上方显示最近的按键事件和已分发的动作"
        }
        "Show the last mouse event above the prompt" => "在提示符上方显示最近的鼠标事件",
        "Mouse capture mode (disabled, simple, smart)" => "鼠标捕获模式（disabled、simple、smart）",
        "Whether to change the mouse cursor shape depending on what is hovered" => {
            "是否根据悬停内容改变鼠标光标形状"
        }
        "Enable or disable the right-click context menu. Disabled by default; enable it to show the copy/cut/paste menu when right-clicking" => {
            "启用或禁用右键上下文菜单。默认禁用；启用后右键点击会显示复制/剪切/粘贴菜单。"
        }
        "Enable or disable the right-click context menu. Disabled by default" => {
            "启用或禁用右键上下文菜单。默认禁用。"
        }
        "Enable or disable the tutorial. Defaults to `true`" => "启用或禁用新手教程。默认为 `true`",
        "Enable or disable auto-suggest (auto-started tab completion suggestions)" => {
            "启用或禁用自动建议（自动启动的 Tab 补全建议）"
        }
        "Enable or disable flycomp for synthesizing shell completions when no useful compspec is found" => {
            "启用或禁用 flycomp，在没有有用的 compspec 时合成 shell 补全"
        }
        "Enable or disable flycomp for synthesizing shell completions" => {
            "启用或禁用 flycomp 合成 shell 补全"
        }
        "How to sort suggestions when fuzzy scores are tied (mtime, alphabetical)" => {
            "模糊分数相同时如何对建议排序（mtime、alphabetical）"
        }
        "Maximum number of suggestion rows to render for tab-completion lists" => {
            "Tab 补全列表可渲染的最大建议行数"
        }
        "Directory where flycomp output should be saved. You should source the completions from this directory in your bashrc so flyline can use them next time" => {
            "flycomp 输出应保存到的目录。应在 bashrc 中 source 该目录下的补全，以便 flyline 下次使用"
        }
        "Directory where flycomp output should be saved" => "flycomp 输出应保存到的目录",
        "Blacklist of command words for which flycomp prompt should be bypassed" => {
            "应绕过 flycomp 提示的命令词黑名单"
        }
        "Enable or disable sandboxing (bubblewrap/bwrap)" => "启用或禁用沙箱（bubblewrap/bwrap）",
        "Run execution unsandboxed (bypass bubblewrap/bwrap sandboxing)" => {
            "非沙箱运行（绕过 bubblewrap/bwrap 沙箱）"
        }
        "Maximum depth for recursive subcommand synthesis/exploration" => {
            "递归子命令合成/探索的最大深度"
        }
        "Timeout in milliseconds for running commands during synthesis" => {
            "合成期间运行命令的超时毫秒数"
        }
        "Parsing strategy (man-page-then-run-help, man-page, run-help, man-page-or-run-help)" => {
            "解析策略（man-page-then-run-help、man-page、run-help、man-page-or-run-help）"
        }
        "Block until the command finishes, optionally with a timeout in milliseconds. With no value, polls indefinitely (i32::MAX ms ≈ 24.8 days).  If the timeout expires the command continues running in the background and subsequent renders will pick up its output" => {
            "阻塞直到命令完成，可选地指定毫秒超时。无值时无限轮询（i32::MAX 毫秒 ≈ 24.8 天）。如果超时，命令继续在后台运行，后续渲染会获取其输出"
        }
        "What to show while the command is running.  Either a number (spaces) or 'prev' (use the previous output of the command)" => {
            "命令运行期间显示的内容。可以是数字（空格数）或 'prev'（使用命令上一次的输出）"
        }
        "Name to embed in prompt strings as the animation placeholder" => {
            "嵌入提示符字符串中作为动画占位符的名称"
        }
        "Name to embed in prompt strings as the widget placeholder" => {
            "嵌入提示符字符串中作为组件占位符的名称"
        }
        "Name to embed in prompt strings as the widget placeholder. Defaults to `FLYLINE_COPY_BUFFER`" => {
            "嵌入提示符字符串中作为组件占位符的名称。默认为 `FLYLINE_COPY_BUFFER`"
        }
        "Name to embed in prompt strings as the widget placeholder. Defaults to `FLYLINE_LAST_COMMAND_DURATION`" => {
            "嵌入提示符字符串中作为组件占位符的名称。默认为 `FLYLINE_LAST_COMMAND_DURATION`"
        }
        "Name to embed in prompt strings as the widget placeholder. Defaults to `FLYLINE_LEADER_MODE`" => {
            "嵌入提示符字符串中作为组件占位符的名称。默认为 `FLYLINE_LEADER_MODE`"
        }
        "Name to embed in prompt strings as the widget placeholder. Defaults to `FLYLINE_MOUSE_MODE`" => {
            "嵌入提示符字符串中作为组件占位符的名称。默认为 `FLYLINE_MOUSE_MODE`"
        }
        "Playback speed in frames per second (default: 10)" => "播放速度（每秒帧数，默认：10）",
        "Reverse direction at each end instead of wrapping (ping-pong / bounce mode)" => {
            "在两端反转方向而不是循环（ping-pong / 弹跳模式）"
        }
        "One or more animation frames (positional).  Use `\\e` for the ESC character" => {
            "一个或多个动画帧（位置参数）。使用 `\\e` 表示 ESC 字符"
        }
        "Text to display in the prompt" => "在提示符中显示的文本",
        "Text to display when mouse capture is enabled" => "鼠标捕获启用时显示的文本",
        "Text to display when mouse capture is disabled" => "鼠标捕获禁用时显示的文本",
        "Text to display when the leader key is active" => "leader 键激活时显示的文本",
        "Text to display when the leader key is inactive" => "leader 键未激活时显示的文本",
        "Enable automatic closing character insertion (e.g. insert `)` after `(`)" => {
            "启用自动闭合字符插入（例如在 `(` 后插入 `)`）"
        }
        "Show inline history suggestions" => "显示内联历史建议",
        "Whether mouse clicks and drags on the command buffer change the cursor position and selection. Default is `true`. When `false`, mouse interaction with the buffer does not change the selection" => {
            "在命令缓冲区上的鼠标点击和拖拽是否改变光标位置和选择。默认为 `true`。为 `false` 时，鼠标与缓冲区的交互不会改变选择"
        }
        "The fuzzy mode to set (all, none, no folders)" => {
            "要设置的模糊模式（all、none、no folders）"
        }
        "Logging level to apply" => "要应用的日志级别",
        "Destination: a file path, `stderr`, or `terminal`" => {
            "目标：文件路径、`stderr` 或 `terminal`"
        }
        "Only show log entries from the last duration (e.g. 5s, 2m, 1h)" => {
            "仅显示最近一段时间内的日志条目（例如 5s、2m、1h）"
        }
        "Only copy log entries from the last duration (e.g. 5s, 2m, 1h)" => {
            "仅复制最近一段时间内的日志条目（例如 5s、2m、1h）"
        }

        // Shell integration value descriptions
        "Send no shell integration codes" => "不发送 shell 集成转义码",
        "Only send the escape codes that report prompt start/end positions" => {
            "仅发送报告提示符起始/结束位置的转义码"
        }
        "Send the full set of shell integration codes: prompt positions, execution start/end codes, and cursor-position reporting" => {
            "发送完整的 shell 集成转义码：提示符位置、执行开始/结束码，以及光标位置报告"
        }

        // Top-level after-help
        "Read more at https://github.com/HalFrgrd/flyline" => {
            "了解更多：https://github.com/HalFrgrd/flyline"
        }

        // Key bindings / completion candidate descriptions
        "Accept all currently shown suggestions" => "接受当前显示的所有建议",
        "Accept inline history suggestion" => "接受内联历史建议",
        "Accept the current Yes/No choice in the flycomp prompt" => {
            "接受 flycomp 提示中当前的“是/否”选择"
        }
        "Accept the current fuzzy history search suggestion and immediately run it" => {
            "接受当前模糊历史搜索建议并立即运行"
        }
        "Accept the current fuzzy history search suggestion for editing" => {
            "接受当前模糊历史搜索建议以进行编辑"
        }
        "Accept the currently selected agent output" => "接受当前选中的 AI 助手输出",
        "Accept the currently selected entry" => "接受当前选中的条目",
        "Accept the currently selected entry for agent commands" => {
            "接受当前选中的条目作为 AI 助手命令"
        }
        "Accept the currently selected suggestion" => "接受当前选中的建议",
        "Activate the leader key state" => "激活 leader 键状态",
        "Cancel the current command or exit if no command is running" => {
            "取消当前命令；如果没有命令在运行则退出"
        }
        "Clear the screen" => "清屏",
        "Clear the text buffer" => "清空文本缓冲区",
        "Comment out the current line and submit" => "注释掉当前行并提交",
        "Copy the current text selection to the system clipboard via OSC 52" => {
            "通过 OSC 52 将当前文本选择复制到系统剪贴板"
        }
        "Cut the current text selection: copy it to the clipboard via OSC 52 and delete it from the buffer" => {
            "剪切当前文本选择：通过 OSC 52 复制到剪贴板并从缓冲区删除"
        }
        "Deactivate the leader key state" => "取消激活 leader 键状态",
        "Delete character after cursor" => "删除光标后的字符",
        "Delete character before cursor" => "删除光标前的字符",
        "Delete one word part to the left stopping at punctuation or path segment boundaries" => {
            "按标点或路径段边界删除左侧一个词片段"
        }
        "Delete one word part to the right stopping at punctuation or path segment boundaries" => {
            "按标点或路径段边界删除右侧一个词片段"
        }
        "Delete one word to the left using whitespace as delimiter" => {
            "以空白为分隔删除左侧一个单词"
        }
        "Delete one word to the right using whitespace as delimiter" => {
            "以空白为分隔删除右侧一个单词"
        }
        "Delete until end of line" => "删除到行尾",
        "Delete until start of line" => "删除到行首",
        "Deselect the currently selected agent output entry" => {
            "取消选中当前选中的 AI 助手输出条目"
        }
        "Do nothing (useful for unbinding a key)" => "不执行任何操作（可用于取消绑定按键）",
        "Insert a literal string of characters" => "插入一个字面字符串",
        "Insert a newline" => "插入换行",
        "Insert character" => "插入字符",
        "Insert the last word from the previous command in history" => {
            "插入历史中上一条命令的最后一个词"
        }
        "Move cursor down one line" => "光标下移一行",
        "Move cursor down one line, extending the text selection" => "光标下移一行并扩展文本选择",
        "Move cursor left" => "光标左移",
        "Move cursor left, extending the text selection" => "光标左移并扩展文本选择",
        "Move cursor right" => "光标右移",
        "Move cursor right, extending the text selection" => "光标右移并扩展文本选择",
        "Move cursor to end of line" => "光标移到行尾",
        "Move cursor to end of line, extending the text selection" => "光标移到行尾并扩展文本选择",
        "Move cursor to start of line" => "光标移到行首",
        "Move cursor to start of line, extending the text selection" => {
            "光标移到行首并扩展文本选择"
        }
        "Move cursor up one line" => "光标上移一行",
        "Move cursor up one line, extending the text selection" => "光标上移一行并扩展文本选择",
        "Move down in agent output selection" => "在 AI 助手输出选择中向下移动",
        "Move down in tab completion suggestions" => "在 Tab 补全建议中向下移动",
        "Move left in tab completion suggestions" => "在 Tab 补全建议中向左移动",
        "Move one page down / one column right in tab completion suggestions" => {
            "在 Tab 补全建议中下翻一页 / 右移一列"
        }
        "Move one page up / one column left in tab completion suggestions" => {
            "在 Tab 补全建议中上翻一页 / 左移一列"
        }
        "Move one word left (whitespace delimiter), extending the text selection" => {
            "按空白分隔左移一个单词并扩展文本选择"
        }
        "Move one word left using whitespace as delimiter" => "以空白为分隔左移一个单词",
        "Move one word part left, extending the text selection" => "左移一个词片段并扩展文本选择",
        "Move one word part right, extending the text selection" => "右移一个词片段并扩展文本选择",
        "Move one word part to the left, stopping at punctuation or path segment boundaries" => {
            "按标点或路径段边界左移一个词片段"
        }
        "Move one word part to the right, stopping at punctuation or path segment boundaries" => {
            "按标点或路径段边界右移一个词片段"
        }
        "Move one word right (whitespace delimiter), extending the text selection" => {
            "按空白分隔右移一个单词并扩展文本选择"
        }
        "Move one word right using whitespace as delimiter" => "以空白为分隔右移一个单词",
        "Move right in tab completion suggestions" => "在 Tab 补全建议中向右移动",
        "Move selection to the leftmost directory segment in the prompt" => {
            "将选择移动到提示符中最左侧的目录段"
        }
        "Move selection to the rightmost (current) directory segment in the prompt" => {
            "将选择移动到提示符中最右侧（当前）的目录段"
        }
        "Move to the next tab completion suggestion" => "移动到下一个 Tab 补全建议",
        "Move to the previous tab completion suggestion" => "移动到上一个 Tab 补全建议",
        "Move up in agent output selection" => "在 AI 助手输出选择中向上移动",
        "Move up in tab completion suggestions" => "在 Tab 补全建议中向上移动",
        "Navigate to next history entry" => "导航到下一条历史记录",
        "Navigate to previous history entry" => "导航到上一条历史记录",
        "Navigate to the child directory segment or exit prompt CWD edit mode" => {
            "导航到子目录段或退出提示符目录编辑模式"
        }
        "Navigate to the parent directory segment in the prompt" => "导航到提示符中的父目录段",
        "Paste from the system clipboard" => "从系统剪贴板粘贴",
        "Redo last action" => "重做上一个操作",
        "Replace the buffer with `cd <selected path>` and exit prompt CWD edit mode" => {
            "用 `cd <选中路径>` 替换缓冲区并退出提示符目录编辑模式"
        }
        "Return to the normal command editing mode" => "返回普通命令编辑模式",
        "Run a Bash command" => "运行 Bash 命令",
        "Run the agent mode command" => "运行 AI 助手模式命令",
        "Run the agent mode help command" => "运行 AI 助手模式帮助命令",
        "Scroll down one page" => "向下滚动一页",
        "Scroll down through fuzzy history search results" => "向下滚动模糊历史搜索结果",
        "Scroll up one page" => "向上滚动一页",
        "Scroll up through fuzzy history search results" => "向上滚动模糊历史搜索结果",
        "Select the entire command buffer" => "选择整个命令缓冲区",
        "Select the first entry in agent output selection" => "选择 AI 助手输出选择中的第一个条目",
        "Select the top entry in the fuzzy history search results" => {
            "选择模糊历史搜索结果中的顶部条目"
        }
        "Send EOF to Bash if ignoreeof is non-zero" => "如果 ignoreeof 非零则向 Bash 发送 EOF",
        "Start agent mode with the current buffer again" => "使用当前缓冲区重新启动 AI 助手模式",
        "Start fuzzy search through cancelled command history" => "启动对已取消命令历史的模糊搜索",
        "Start fuzzy search through command history" => "启动对命令历史的模糊搜索",
        "Start prompt directory selection mode, allowing navigation via the prompt's directory segments" => {
            "启动提示符目录选择模式，可通过提示符的目录段导航"
        }
        "Start tab completion" => "启动 Tab 补全",
        "Submit the current command or insert a newline if the buffer is an incomplete expression" => {
            "提交当前命令；如果缓冲区是未完成的表达式则插入换行"
        }
        "Temporarily dismiss the inline history suggestion" => "暂时忽略内联历史建议",
        "Toggle mouse state (Simple and Smart modes)" => "切换鼠标状态（Simple 和 Smart 模式）",
        "Toggle Yes/No choice in the flycomp prompt" => "在 flycomp 提示中切换“是/否”选择",
        "Undo last action" => "撤销上一个操作",

        // Context variables
        "Agent mode failed and is showing an error message" => "AI 助手模式失败并正在显示错误消息",
        "Agent mode finished and is showing a list of selectable suggestions" => {
            "AI 助手模式已完成并正在显示可选择的建议列表"
        }
        "Agent output selection is active and a suggestion is currently selected" => {
            "AI 助手输出选择处于活动状态且当前选中了一个建议"
        }
        "Agent output selection is active and no suggestion is currently selected" => {
            "AI 助手输出选择处于活动状态且当前未选中任何建议"
        }
        "Always true; the catch-all context for unconditional bindings" => {
            "始终为真；无条件绑定的通用上下文"
        }
        "An inline history suggestion is available to be accepted" => "有一条可接受的内联历史建议",
        "Cursor is at the end of the buffer" => "光标位于缓冲区末尾",
        "Cursor is at the end of the trimmed buffer" => "光标位于去除空白后的缓冲区末尾",
        "Cursor is at the start of the buffer" => "光标位于缓冲区开头",
        "Cursor is on the final line of the buffer" => "光标位于缓冲区的最后一行",
        "Cursor is on the first line of the buffer" => "光标位于缓冲区的第一行",
        "Flycomp completion synthesis finished and has a result or error" => {
            "flycomp 补全合成已完成并有结果或错误"
        }
        "Flycomp completion synthesis is currently running in the background" => {
            "flycomp 补全合成正在后台运行"
        }
        "Fuzzy history search overlay for agent commands is active" => {
            "针对 AI 助手命令的模糊历史搜索浮层处于活动状态"
        }
        "Fuzzy history search overlay for cancelled commands is active" => {
            "针对已取消命令的模糊历史搜索浮层处于活动状态"
        }
        "Fuzzy history search overlay for normal commands is active" => {
            "针对普通命令的模糊历史搜索浮层处于活动状态"
        }
        "Fuzzy history search overlay is active" => "模糊历史搜索浮层处于活动状态",
        "Fuzzy history search overlay is active and no entry is currently selected" => {
            "模糊历史搜索浮层处于活动状态且当前未选中任何条目"
        }
        "Prompt directory selection mode is active" => "提示符目录选择模式处于活动状态",
        "Prompting the user whether they want to run flycomp" => "正在询问用户是否要运行 flycomp",
        "Tab completion overlay has at least one candidate and a selected entry" => {
            "Tab 补全浮层至少有一个候选并选中了一个条目"
        }
        "Tab completion overlay is active (any state)" => "Tab 补全浮层处于活动状态（任意状态）",
        "Tab completion overlay is active and has at least one candidate" => {
            "Tab 补全浮层处于活动状态且至少有一个候选"
        }
        "Tab completion overlay is active and has exactly one filtered candidate" => {
            "Tab 补全浮层处于活动状态且恰好有一个过滤后的候选"
        }
        "Tab completion overlay is active and has no candidates at all" => {
            "Tab 补全浮层处于活动状态且没有任何候选"
        }
        "Tab completion overlay is active but fuzzy filtering has no matches" => {
            "Tab 补全浮层处于活动状态但模糊过滤没有匹配项"
        }
        "Tab completion overlay is showing more than one column of candidates" => {
            "Tab 补全浮层显示超过一列候选"
        }
        "Tab completion was triggered by the user (not auto-started)" => {
            "Tab 补全由用户触发（非自动启动）"
        }
        "The command buffer contains at least one newline" => "命令缓冲区至少包含一个换行",
        "The command buffer is empty" => "命令缓冲区为空",
        "The command buffer starts with an agent mode prefix" => "命令缓冲区以 AI 助手模式前缀开头",
        "The content mode is normal editing (no overlay is active)" => {
            "内容模式为普通编辑（无浮层处于活动状态）"
        }
        "The leader key is currently active" => "leader 键当前处于活动状态",
        "There is an active text selection in the buffer" => "缓冲区中有活动的文本选择",
        "Waiting for tab completion candidates to be produced" => "正在等待生成 Tab 补全候选",
        "Waiting for the agent mode subprocess to finish" => "正在等待 AI 助手模式子进程完成",

        // Palette style descriptions
        "Default style for unclassified command buffer text" => "未分类命令缓冲区文本的默认样式",
        "Dimmed style for secondary and decorative text" => "次要和装饰性文本的变暗样式",
        "Highlight style for characters matched by fuzzy search" => "模糊搜索匹配字符的高亮样式",
        "Highlight style for the current text selection" => "当前文本选择的高亮样式",
        "Rainbow bracket/quote colour for nesting depth 1 (outermost)" => {
            "嵌套深度 1（最外层）的彩虹括号/引号颜色"
        }
        "Rainbow bracket/quote colour for nesting depth 2" => "嵌套深度 2 的彩虹括号/引号颜色",
        "Rainbow bracket/quote colour for nesting depth 3" => "嵌套深度 3 的彩虹括号/引号颜色",
        "Rainbow bracket/quote colour for nesting depth 4" => "嵌套深度 4 的彩虹括号/引号颜色",
        "Style for inline code spans in Markdown" => "Markdown 内联代码片段的样式",
        "Style for inline history suggestions shown after the cursor" => {
            "光标后显示的内联历史建议的样式"
        }
        "Style for level-1 Markdown headings (# heading)" => "Markdown 一级标题（# 标题）的样式",
        "Style for level-2 Markdown headings (## heading)" => "Markdown 二级标题（## 标题）的样式",
        "Style for level-3 Markdown headings (### heading)" => {
            "Markdown 三级标题（### 标题）的样式"
        }
        "Style for matched opening/closing bracket or quote pairs" => {
            "匹配的左右括号或引号对的样式"
        }
        "Style for the right click context menu background" => "右键上下文菜单背景的样式",
        "Style for tutorial hint text" => "教程提示文本的样式",
        "Style used to render key sequences in the UI" => "界面中渲染按键序列的样式",
        "Syntax highlighting for bash reserved words (e.g. if, while, for)" => {
            "Bash 保留字（例如 if、while、for）的语法高亮"
        }
        "Syntax highlighting for double-quoted strings" => "双引号字符串的语法高亮",
        "Syntax highlighting for environment variable references (e.g. $HOME)" => {
            "环境变量引用（例如 $HOME）的语法高亮"
        }
        "Syntax highlighting for recognised shell commands (e.g. ls, git)" => {
            "已识别 shell 命令（例如 ls、git）的语法高亮"
        }
        "Syntax highlighting for shell comments (text after #)" => {
            "shell 注释（# 后的文本）的语法高亮"
        }
        "Syntax highlighting for single-quoted strings" => "单引号字符串的语法高亮",
        "Syntax highlighting for unrecognised commands" => "未识别命令的语法高亮",
        "Syntax highlighting for unrecognised environment variable references" => {
            "未识别环境变量引用的语法高亮"
        }

        _ => return None,
    })
}

/// Localize the clap help template labels (Usage/Options/...).
fn zh_help_template() -> &'static str {
    "\
{before-help}{about-with-newline}
用法：{usage}

{all-args}{after-help}\
"
}

/// Rewrites a clap [`Command`] tree with Simplified Chinese help text when the
/// active language is Chinese.  Strings without a translation keep their
/// English text.
pub(crate) fn localize_clap_command(cmd: Command) -> Command {
    if current_language() != Language::Zh {
        return cmd;
    }

    fn localize_subcommand(sub: Command) -> Command {
        let about = sub
            .get_about()
            .map(|s| s.to_string())
            .and_then(|s| zh_help_translation(&s))
            .map(|zh| clap::builder::StyledStr::from(zh));
        let long_about = sub
            .get_long_about()
            .map(|s| s.to_string())
            .and_then(|s| zh_help_translation(&s))
            .map(|zh| clap::builder::StyledStr::from(zh));
        sub.about(
            about
                .map(clap::builder::Resettable::Value)
                .unwrap_or(clap::builder::Resettable::Reset),
        )
        .long_about(
            long_about
                .map(clap::builder::Resettable::Value)
                .unwrap_or(clap::builder::Resettable::Reset),
        )
        .help_template(zh_help_template())
        .mut_args(localize_arg)
        .mut_subcommands(localize_subcommand)
    }

    fn localize_arg(arg: clap::Arg) -> clap::Arg {
        let arg = if let Some(zh) = arg
            .get_help()
            .map(|s| s.to_string())
            .and_then(|s| zh_help_translation(&s))
        {
            arg.help(clap::builder::Resettable::Value(
                clap::builder::StyledStr::from(zh),
            ))
        } else {
            // No translation for this help text: keep the original English
            // text instead of blanking the description entirely.
            arg
        };
        if arg.get_help_heading().is_none() {
            arg.help_heading(Some("选项"))
        } else {
            arg
        }
    }

    let after_help = cmd
        .get_after_help()
        .map(|s| s.to_string())
        .and_then(|s| zh_help_translation(&s))
        .map(|zh| clap::builder::StyledStr::from(zh));
    cmd.help_template(zh_help_template())
        .after_help(
            after_help
                .map(clap::builder::Resettable::Value)
                .unwrap_or(clap::builder::Resettable::Reset),
        )
        .subcommand_help_heading("命令")
        .mut_args(localize_arg)
        .mut_subcommands(localize_subcommand)
}

fn parse_language(lang: &str) -> Option<Language> {
    let lang = lang.to_ascii_lowercase();
    if lang.starts_with("zh") {
        Some(Language::Zh)
    } else if lang.starts_with("en") || lang == "c" || lang == "posix" {
        Some(Language::En)
    } else {
        None
    }
}

fn detect_language() -> Language {
    // Explicit user override first.
    if let Some(lang) = crate::bash_funcs::get_envvar_value("FLYLINE_LANG")
        && let Some(language) = parse_language(&lang)
    {
        return language;
    }
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(lang) = crate::bash_funcs::get_envvar_value(var)
            && let Some(language) = parse_language(&lang)
        {
            return language;
        }
    }
    Language::En
}

#[macro_export]
macro_rules! t {
    ($s:literal) => {
        $crate::i18n::translate($s)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthChar;

    #[test]
    fn detects_chinese_language_variants() {
        assert_eq!(parse_language("zh"), Some(Language::Zh));
        assert_eq!(parse_language("zh_CN"), Some(Language::Zh));
        assert_eq!(parse_language("zh-CN.UTF-8"), Some(Language::Zh));
    }

    #[test]
    fn detects_english_and_default_locales() {
        assert_eq!(parse_language("en"), Some(Language::En));
        assert_eq!(parse_language("en_US.UTF-8"), Some(Language::En));
        assert_eq!(parse_language("C"), Some(Language::En));
        assert_eq!(parse_language("ja_JP"), None);
    }

    #[test]
    fn unknown_keys_fall_back_to_english() {
        assert_eq!(translate("No such translation"), "No such translation");
    }

    #[test]
    fn known_keys_translate_to_chinese() {
        assert_eq!(
            zh_translation("Press Enter to start the tutorial"),
            Some("按 Enter 开始教程")
        );
        assert_eq!(
            zh_help_translation("Show version information"),
            Some("显示版本信息")
        );
    }

    #[test]
    fn right_click_menu_help_translations_match() {
        assert_eq!(
            zh_help_translation(
                "Enable or disable the right-click context menu. Disabled by default; enable it to show the copy/cut/paste menu when right-clicking"
            ),
            Some("启用或禁用右键上下文菜单。默认禁用；启用后右键点击会显示复制/剪切/粘贴菜单。")
        );
        assert_eq!(
            zh_help_translation(
                "Enable or disable the right-click context menu. Disabled by default"
            ),
            Some("启用或禁用右键上下文菜单。默认禁用。")
        );
    }

    #[test]
    fn dump_character_widths_for_diagnostics() {
        for ch in ['你', '好', '。', '●', '✅', '⚠', '💡', '•', '—', '…', '·'] {
            println!(
                "U+{:04X} {} width={}",
                ch as u32,
                ch,
                ch.width().unwrap_or(0)
            );
        }
    }

    #[test]
    fn completion_descriptions_have_chinese_translations() {
        let keys = [
            "Move cursor left",
            "Accept the currently selected suggestion",
            "Syntax highlighting for recognised shell commands (e.g. ls, git)",
            "Tab completion overlay is active and has at least one candidate",
            "Always true; the catch-all context for unconditional bindings",
            "Style for matched opening/closing bracket or quote pairs",
            "Run a Bash command",
            "⎘ Copy (buffer)",
            "↶ Undo",
            "alias: {}",
            "builtin: {}",
        ];
        for key in keys {
            assert!(
                zh_help_translation(key).is_some() || zh_translation(key).is_some(),
                "missing translation for: {key}"
            );
        }
    }
}
