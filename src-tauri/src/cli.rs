use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "kterm",
    author,
    version,
    about = "Scriptable Windows Terminal",
    help_template = "{name} v{version}\n{about}\n\n{usage-heading} {usage}\n\n{all-args}\n{after-help}",
    after_help = r#"EXAMPLES:
  Declarative YAML session engine:
    kterm --apply dev-session.yaml
    kterm dev-session.yaml --suffix -backend
    kterm dev-session.yaml --suffix-auto
    kterm dev-session.yaml --dry-run
    kterm --export-layout layout.yaml

  Spawn tab in current or target window:
    kterm --profile powershell
    kterm --profile cmd --window win-1
    kterm --new-window

  Split panes (returns new session ID):
    kterm --select-tab tab1 --split-down --profile wsl
    kterm --select-tab tab1 --split-right --profile git-bash
    kterm --select-tab tab1 --split-left --profile powershell
    kterm --select-tab tab1 --split-up --profile cmd

  Send unquoted commands / text:
    kterm --select-tab tab1 --send-text git status
    kterm --select-tab tab1 --send-text npm run dev

  Tab titles, badges, colors, & window title:
    kterm --select-tab tab1 --send-title Server Logs
    kterm --select-tab tab1 --set-badge PROD
    kterm --select-tab tab1 --set-color #E53935
    kterm --window win-1 --set-window-title Main Workspace

  Layout management:
    kterm --select-tab tab1 --unsplit
    kterm --select-tab tab1 --explode-split

  Listing & queries:
    kterm --list-windows
    kterm --list-tabs --window win-1 --json

  Closing tabs & windows:
    kterm --select-tab tab1 --close --force
    kterm --close-window win-1"#
)]
pub struct CliArgs {
    #[arg(short, long, help = "Terminal profile (powershell, cmd, wsl, git-bash)")]
    pub profile: Option<String>,

    #[arg(long, help = "Run shell with Administrator privileges")]
    pub admin: bool,

    #[arg(long, help = "Internal elevated PTY bridge mode")]
    pub elevated_pty_bridge: Option<String>,

    #[arg(long, help = "Target window ID (e.g. win-1, win-2)")]
    pub window: Option<String>,

    #[arg(long, help = "Force spawn in a new GUI window")]
    pub new_window: bool,

    #[arg(long, help = "List all active GUI windows")]
    pub list_windows: bool,

    #[arg(long, help = "List active tabs")]
    pub list_tabs: bool,

    #[arg(long, help = "Format output as JSON")]
    pub json: bool,

    #[arg(
        long,
        num_args = 1..,
        help = "Select target tab by ID or Title"
    )]
    pub select_tab: Vec<String>,

    #[arg(
        long,
        num_args = 1..,
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "Send unquoted text/command to selected tab"
    )]
    pub send_text: Option<Vec<String>>,

    #[arg(
        long,
        num_args = 1..,
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "Set unquoted title for selected tab"
    )]
    pub send_title: Option<Vec<String>>,

    #[arg(
        long,
        num_args = 1..,
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "Set unquoted title for target window"
    )]
    pub set_window_title: Option<Vec<String>>,

    #[arg(long, help = "Set visual badge for selected tab")]
    pub set_badge: Option<String>,

    #[arg(long, help = "Set accent color for selected tab")]
    pub set_color: Option<String>,

    #[arg(long, help = "Bring target window/tab to front")]
    pub focus: bool,

    #[arg(long, help = "Close selected tab")]
    pub close: bool,

    #[arg(long, help = "Close specified GUI window by ID")]
    pub close_window: Option<String>,

    #[arg(long, help = "Force close without prompt")]
    pub force: bool,

    #[arg(long, help = "Split selected tab horizontally to the right")]
    pub split_right: bool,

    #[arg(long, help = "Split selected tab horizontally to the left")]
    pub split_left: bool,

    #[arg(long, help = "Split selected tab vertically downward")]
    pub split_down: bool,

    #[arg(long, help = "Split selected tab vertically upward")]
    pub split_up: bool,

    #[arg(long, help = "Move existing tab ID into split layout")]
    pub move_tab: Option<String>,

    #[arg(long, help = "Detach split pane back to standalone tab")]
    pub unsplit: bool,

    #[arg(long, help = "Separate all panes in split layout into standalone tabs")]
    pub explode_split: bool,

    #[arg(long, help = "Path to YAML session file to load and apply")]
    pub apply: Option<String>,

    #[arg(long, help = "Export current window layout to specified .yaml file")]
    pub export_layout: Option<String>,

    #[arg(long, help = "Append static text to window.id")]
    pub suffix: Option<String>,

    #[arg(long, help = "Automatically append first unused numeric suffix (-1, -2, -3)")]
    pub suffix_auto: bool,

    #[arg(long, help = "Validate YAML syntax without opening windows or modifying state")]
    pub dry_run: bool,

    #[arg(long, help = "Read recent output lines from tab (optional tab ID or uses --select-tab)")]
    pub read_text: Option<Option<String>>,

    #[arg(long, help = "Number of tail lines to read for --read-text (default: 50)")]
    pub tail: Option<usize>,

    #[arg(long, help = "Keep raw ANSI codes when reading output")]
    pub raw: bool,

    #[arg(long, help = "Wait until tab output matches string pattern or regex")]
    pub wait_for: Option<String>,

    #[arg(long, help = "Include past output history when checking --wait-for pattern")]
    pub from_history: bool,

    #[arg(long, help = "Wait until tab returns to shell prompt")]
    pub wait_for_prompt: bool,

    #[arg(long, help = "Timeout in seconds for --wait-for (default: 30)")]
    pub timeout: Option<u64>,

    #[arg(long, help = "Run in host daemon mode")]
    pub daemon: bool,
}

pub fn print_help() {
    println!(
        r#"kterm v0.1.0 — Scriptable Windows Terminal

Usage: kterm.exe [OPTIONS]

Options:
      --apply <FILE>                  Path to YAML session file to load and apply
      --export-layout <FILE>          Export current window layout to specified .yaml file
      --suffix <TEXT>                 Append static text to window.id
      --suffix-auto                   Automatically append first unused numeric suffix (-1, -2, -3)
      --dry-run                       Validate YAML syntax without making changes
  -p, --profile <PROFILE>             Terminal profile (powershell, cmd, wsl, git-bash)
      --window <WINDOW>               Target window ID (e.g. win-1, win-2)
      --new-window                    Force spawn in a new GUI window
      --list-windows                  List all active GUI windows
      --list-tabs                     List active tabs
      --json                          Format output as JSON
      --select-tab <SELECT_TAB>...    Select target tab by ID or Title
      --send-text <SEND_TEXT>...      Send unquoted text/command to selected tab
      --send-title <SEND_TITLE>...    Set unquoted title for selected tab
      --set-window-title <TITLE>...   Set unquoted title for target window
      --set-badge <SET_BADGE>         Set visual badge for selected tab
      --set-color <SET_COLOR>         Set accent color for selected tab
      --focus                         Bring target window/tab to front
      --close                         Close selected tab
      --close-window <CLOSE_WINDOW>   Close specified GUI window by ID
      --force                         Force close without prompt
      --split-right                   Split selected tab horizontally to the right
      --split-left                    Split selected tab horizontally to the left
      --split-down                    Split selected tab vertically downward
      --split-up                      Split selected tab vertically upward
      --move-tab <MOVE_TAB>           Move existing tab ID into split layout
      --unsplit                       Detach split pane back to standalone tab
      --explode-split                 Separate all panes in split layout into standalone tabs
      --read-text [TAB_ID]            Read recent output lines from tab (default: 50 lines)
      --tail <N>                      Number of tail lines to read (default: 50)
      --raw                           Keep raw ANSI escape sequences when reading output
      --wait-for <PATTERN>            Wait until tab output matches string pattern or regex
      --wait-for-prompt               Wait until tab returns to shell prompt
      --timeout <SECONDS>             Timeout in seconds for --wait-for (default: 30)
      --daemon                        Run in host daemon mode
  -h, --help                          Print help
  -V, --version                       Print version

EXAMPLES:
  Declarative YAML session engine:
    kterm --apply dev-session.yaml
    kterm dev-session.yaml --suffix -backend
    kterm dev-session.yaml --suffix-auto
    kterm dev-session.yaml --dry-run
    kterm --export-layout layout.yaml

  Spawn tab in current or target window:
    kterm --profile powershell
    kterm --profile cmd --window win-1
    kterm --new-window

  Split panes:
    kterm --select-tab tab1 --split-down --profile wsl
    kterm --select-tab tab1 --split-right --profile git-bash

  Send unquoted commands / text:
    kterm --select-tab tab1 --send-text git status
    kterm --select-tab tab1 --send-text npm run dev

  Output history & synchronization:
    kterm --select-tab tab1 --read-text
    kterm --select-tab tab1 --read-text --tail 10
    kterm --select-tab tab1 --send-text echo hello`r
    kterm --select-tab tab1 --wait-for hello
    kterm --select-tab tab1 --wait-for-prompt

  Tab titles, badges, colors, & window title:
    kterm --select-tab tab1 --send-title Server Logs
    kterm --select-tab tab1 --set-badge PROD
    kterm --select-tab tab1 --set-color #E53935
    kterm --window win-1 --set-window-title Main Workspace"#
    );
}
