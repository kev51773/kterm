use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "kterm",
    author,
    version,
    about = "Scriptable Windows Terminal",
    disable_help_flag = false
)]
pub struct CliArgs {
    #[arg(short, long, help = "Terminal profile (powershell, cmd, wsl, git-bash)")]
    pub profile: Option<String>,

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

    #[arg(long, help = "Force close without prompt")]
    pub force: bool,
}
