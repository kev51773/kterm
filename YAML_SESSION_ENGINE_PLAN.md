# Declarative YAML Session Engine Implementation Spec

> **For AI Agents:** This specification contains the complete technical blueprint for implementing the Declarative YAML Session Engine in `kterm`. Follow every step and specification exactly.

---

## 1. Project Context & Architecture Overview

`kterm` is a scriptable Windows Terminal application built with **Tauri v2 (Rust backend + TypeScript/Vite frontend)**.

It runs in two modes:
1. **Host Daemon (`kterm --daemon`):** Single background process running a local HTTP server (`http://127.0.0.1:9999`) and Tauri GUI webview windows (`win-1`, `win-2`, etc.).
2. **CLI Client Mode (`kterm [OPTIONS]`):** Command-line client that communicates with the local host daemon via HTTP REST endpoints.

### Key Workspace Files
- `src-tauri/src/main.rs`: Main binary entry point.
- `src-tauri/src/cli.rs`: Command line arguments definition (`clap::Parser`).
- `src-tauri/src/client.rs`: CLI client command dispatcher.
- `src-tauri/src/daemon/mod.rs`: HTTP REST server endpoints & state management (`AppState`, `pty_manager`, `window_layouts`).
- `src-tauri/src/exporter.rs`: Layout export logic.
- `src-tauri/src/pty/layout.rs`: `LayoutNode` enum (Pane vs Split tree representation).
- `src/main.ts`: TypeScript frontend UI & event listeners.

---

## 2. Detailed Technical Requirements

### 2.1 Dependencies (`Cargo.toml`)
Add `serde_yaml = "0.9"` to `src-tauri/Cargo.toml`.

### 2.2 YAML Session Schema (`src-tauri/src/yaml.rs`)
Create `src-tauri/src/yaml.rs` defining the YAML session schema:

```yaml
window:
  id: dev-win
  title: "Main Workspace"

tabs:
  - profile: wsl # options: powershell, cmd, wsl, git-bash
    id: server
    title: "Backend API"
    badge: "DEV"
    color: "#4CAF50"
    cwd: "C:\\Projects\\api"
    send_text: |
      echo "hello world" > out.txt
      this is tricky: /"!"$%$%&^&&*)()__+-=-[]{};':@,./<>?
    splits:
      - direction: down # options: down, right, up, left
        profile: wsl
        id: logs
        cwd: "C:\\Projects\\api\\logs"
        send_text: "tail -f app.log"
        splits:
          - direction: right
            profile: cmd
            id: monitor
            send_text: "systeminfo"
```

#### Rust Schema Definition (`yaml.rs`):
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlSessionSpec {
    pub window: YamlWindowSpec,
    pub tabs: Vec<YamlTabSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlWindowSpec {
    pub id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlTabSpec {
    pub id: Option<String>,
    pub profile: String,
    pub title: Option<String>,
    pub badge: Option<String>,
    pub color: Option<String>,
    pub cwd: Option<String>,
    pub send_text: Option<String>,
    pub splits: Option<Vec<YamlSplitSpec>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlSplitSpec {
    pub direction: String, // "down", "right", "left", "up"
    pub profile: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub badge: Option<String>,
    pub color: Option<String>,
    pub cwd: Option<String>,
    pub send_text: Option<String>,
    pub splits: Option<Vec<YamlSplitSpec>>,
}
```

---

### 2.3 CLI Flags & Option Changes (`cli.rs` & `main.rs`)

1. **New CLI Flags:**
   - `--apply <FILE>`: Path to YAML session file to load and apply.
   - `--export-layout <FILE>`: Export current window layout to specified `.yaml` file.
   - `--suffix <TEXT>`: Append static text to `window.id`.
   - `--suffix-auto`: Automatically scan active windows and append first unused numeric suffix (`-1`, `-2`, `-3`).
   - `--dry-run`: Validate YAML syntax, profile names, and window collisions without making changes or opening windows.

2. **Removed CLI Flags:**
   - Remove `--export-script` and `--format` flags completely. Also remove all the related old script generation code.

3. **Updated `print_help()` (`cli.rs`):**
   Update help text with all new commands, options, and YAML usage examples.

---

### 2.4 Strict CLI Error Validation Rules

1. **Window Creation Collision Rule:**
   - If target `window.id` (after applying `--suffix` or `--suffix-auto`) already exists on daemon, `kterm --apply` fails immediately with:
     `Error: Window '<window_id>' already exists.`
   - (Unless `--suffix-auto` is set, which automatically picks next unused ID `$id-$N`).

2. **Tab Selection Ambiguity Rule:**
   When targeting a tab by ID/Title via `kterm --select-tab <target>` without `--window`:
   - **Exactly 1 match across windows:** Succeeds.
   - **Multiple matches in different windows:** Fails immediately with error:
     `Error: Ambiguous tab '<target>' found in windows '<win1>', '<win2>'. Specify --window <win_id>.`
   - **0 matches:** Fails immediately with error:
     `Error: Tab '<target>' not found.`

---

### 2.5 Exporter Subsystem (`exporter.rs`)

Replace legacy `.ps1`, `.sh`, `.bat` code (~600 lines) with clean YAML serializer `export_yaml_layout`:
- Queries `state.pty_manager` and `state.window_layouts` for the target window.
- Serializes layout tree into clean, human-readable YAML format.
- Output includes `window`, `tabs`, `splits`, `title`, `badge`, `color`, and `cwd`.

---

### 2.6 Frontend UI Update (`src/main.ts` & `index.html`)

- Update `export-script-btn` click listener in `src/main.ts`.
- Save dialog defaults to `kterm-layout.yaml` filter.
- Calls daemon endpoint `/export-layout?window=<window_id>`.
- Removes old format modal selection (only `.yaml` is supported).

---

## 3. Step-by-Step Implementation Roadmap

1. **Step 1: Cargo Dependency & Schema (`Cargo.toml` & `yaml.rs`)**
   - Add `serde_yaml = "0.9"` to `Cargo.toml`.
   - Create `src-tauri/src/yaml.rs` with parser, validator, and applicator logic.

2. **Step 2: CLI Options & Help (`cli.rs` & `main.rs`)**
   - Add `--apply`, `--export-layout`, `--suffix`, `--suffix-auto`, `--dry-run`.
   - Remove `--export-script` and `--format`.
   - Add `.yaml`/`.yml` direct argument interception in `main.rs`.
   - Update `print_help()` with full YAML documentation.

3. **Step 3: Client & Daemon HTTP Handlers (`client.rs` & `daemon/mod.rs`)**
   - Implement HTTP POST `/apply` endpoint in daemon to process `YamlSessionSpec`.
   - Implement `--suffix-auto` resolution logic on daemon.
   - Implement window collision check and ambiguous tab error checks.

4. **Step 4: YAML Exporter (`exporter.rs`)**
   - Implement `export_yaml_layout` returning clean YAML string.

5. **Step 5: Frontend UI Update (`src/main.ts`)**
   - Connect UI Export button directly to `/export-layout` saving `.yaml`.

6. **Step 6: Build & Verification**
   - Run `cargo check`.
   - Rebuild executable via `npx tauri build --no-bundle`.
   - Perform end-to-end verification tests.

---

## 4. Verification Checklist

- [ ] `cargo check` compiles with 0 errors.
- [ ] `npx tauri build --no-bundle` builds `src-tauri/target/release/kterm.exe`.
- [ ] `kterm --help` displays updated CLI options and YAML examples.
- [ ] `kterm --apply test.yaml --dry-run` validates syntax without opening windows.
- [ ] `kterm --apply test.yaml --suffix-auto` creates `dev-win-1`, `dev-win-2`, etc.
- [ ] Duplicate window creation without `--suffix-auto` fails with explicit error.
- [ ] Ambiguous tab selection across multiple windows fails with explicit error.
- [ ] UI Export button saves `.yaml` file cleanly.
