# Phase 2 Implementation Plan - Dual-Mode CLI & Multi-Tab/Window

## Objective
Implement single-instance CLI client execution in `kterm.exe`, build the CLI argument parser with tail-argument capturing, add multi-window & multi-tab state tracking in the Axum daemon, and enable title-based tab resolution.

---

## 1. CLI Parsing Strategy (`src-tauri/src/cli.rs`)

### Dependency Additions (`Cargo.toml`)
```toml
clap = { version = "4.5", features = ["derive"] }
reqwest = { version = "0.12", features = ["json"] }
```

### CLI Structural Definition
```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "kterm", author, version, about = "Scriptable Windows Terminal")]
pub struct CliArgs {
    #[arg(short, long)]
    pub profile: Option<String>,

    #[arg(long)]
    pub window: Option<String>,

    #[arg(long)]
    pub new_window: bool,

    #[arg(long)]
    pub list_windows: bool,

    #[arg(long)]
    pub list_tabs: bool,

    #[arg(long)]
    pub json: bool,

    #[arg(long, num_args = 1..)]
    pub select_tab: Vec<String>, // Tab IDs or Titles

    #[arg(long, num_args = 1.., trailing_var_arg = true)]
    pub send_text: Option<Vec<String>>,

    #[arg(long, num_args = 1.., trailing_var_arg = true)]
    pub send_title: Option<Vec<String>>,

    #[arg(long)]
    pub set_badge: Option<String>,

    #[arg(long)]
    pub set_color: Option<String>,

    #[arg(long)]
    pub focus: bool,

    #[arg(long)]
    pub close: bool,

    #[arg(long)]
    pub force: bool,
}
```

---

## 2. Dual-Mode Entrypoint Logic (`src-tauri/src/main.rs`)

```rust
fn main() {
    let args = CliArgs::parse();

    // Check if daemon is active on 127.0.0.1:9999
    if is_daemon_running() {
        // CLIENT MODE: Dispatch HTTP REST API call to running daemon
        let client = Client::new();
        if let Err(e) = handle_client_mode(&args, &client) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        // Exit immediately after CLI command completes
        std::process::exit(0);
    }

    // HOST MODE: No daemon detected. Spawn Axum server + Tauri GUI window
    run_tauri_host(args);
}
```

---

## 3. Tab Resolution & Multi-Window State (`src-tauri/src/daemon/mod.rs`)

### Resolution Logic (`resolve_tabs`)
```rust
impl DaemonState {
    pub fn resolve_tabs(&self, targets: &[String]) -> Vec<String> {
        let mut result = Vec::new();
        for target in targets {
            // 1. Direct ID match
            if self.tabs.contains_key(target) {
                result.push(target.clone());
                continue;
            }
            // 2. Exact Title match
            for (id, tab) in &self.tabs {
                if tab.title.eq_ignore_ascii_case(target) {
                    result.push(id.clone());
                }
            }
        }
        result
    }
}
```

---

## 4. Verification Checkpoints

1. **Host Launch**: Run `./kterm.exe` -> launches GUI + daemon listening on port 9999.
2. **Client Execution**: In a separate terminal run:
   ```powershell
   # List tabs
   .\kterm.exe --list-tabs
   # Spawn tab with profile
   $id = .\kterm.exe --profile git-bash
   # Target by ID and send raw input
   .\kterm.exe --select-tab $id --send-text echo "Hello World"
   # Target by Title
   .\kterm.exe --select-tab "Git Repo" --send-title "Main Dev Repo"
   ```
3. Verify client processes exit immediately after printing handles/results.
