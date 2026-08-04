use crate::daemon::{AppState, TabInfo};
use crate::pty::LayoutNode;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    PowerShell,
    Batch,
    Shell,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "bat" | "batch" | "cmd" => ExportFormat::Batch,
            "sh" | "bash" | "shell" => ExportFormat::Shell,
            _ => ExportFormat::PowerShell,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionStep {
    CreateRoot {
        tab_id: String,
    },
    Split {
        target_id: String,
        direction: String,
        new_id: String,
    },
}

/// Build and return the script text for the given window.
///
/// Walks the layout tree depth-first to create top-level tabs first, then
/// creates split panes directly using `--select-tab <target> --split-<dir> --profile <profile>`.
/// Finally appends metadata commands (title, badge, colour) for any tabs that have them set.
pub fn export_layout(state: &AppState, window_id: &str, format: ExportFormat) -> String {
    // ── 1. Gather all tabs for this window ──────────────────────────────────
    let sessions = state.pty_manager.list_by_window(Some(window_id));

    // Build a quick lookup: tab_id → TabInfo
    let tab_infos: Vec<TabInfo> = sessions
        .iter()
        .map(|s| TabInfo {
            id: s.id.clone(),
            pid: s.pid,
            profile: s.profile.clone(),
            window_id: s.window_id.clone(),
            title: s.title.lock().unwrap().clone(),
            badge: s.badge.lock().unwrap().clone(),
            color: s.color.lock().unwrap().clone(),
        })
        .collect();

    let find_tab = |id: &str| -> Option<&TabInfo> {
        tab_infos.iter().find(|t| t.id == id)
    };

    // ── 2. Determine execution steps from layout tree ────────────────────────
    let layouts = state.window_layouts.lock().unwrap();
    let win_layouts = match layouts.get(window_id) {
        Some(l) => l.clone(),
        None => vec![],
    };
    drop(layouts);

    let mut steps: Vec<ActionStep> = Vec::new();
    let mut ordered_tab_ids: Vec<String> = Vec::new();

    for node in &win_layouts {
        let root_id = first_tab_id(node);
        if !ordered_tab_ids.contains(&root_id) {
            ordered_tab_ids.push(root_id.clone());
        }
        steps.push(ActionStep::CreateRoot {
            tab_id: root_id,
        });

        collect_node_steps(node, &mut steps, &mut ordered_tab_ids);
    }

    // Fallback for any orphaned tabs not in win_layouts
    for t in &tab_infos {
        if !ordered_tab_ids.contains(&t.id) {
            ordered_tab_ids.push(t.id.clone());
            steps.push(ActionStep::CreateRoot {
                tab_id: t.id.clone(),
            });
        }
    }

    // Build the list of TabInfo in execution order.
    let ordered_tabs: Vec<&TabInfo> = ordered_tab_ids
        .iter()
        .filter_map(|id| find_tab(id))
        .collect();

    // ── 3. Assign variable names & current exe path ──────────────────────────
    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "kterm.exe".to_string());

    let var_names: Vec<(String, String)> = ordered_tab_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), var_name(i + 1, format)))
        .collect();

    let lookup_var = |id: &str| -> String {
        var_names
            .iter()
            .find(|(tid, _)| tid == id)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| id.to_string())
    };

    // ── 4. Render script ─────────────────────────────────────────────────────
    let now = chrono_now();
    let tab_count = ordered_tabs.len();
    let split_count = steps.iter().filter(|s| matches!(s, ActionStep::Split { .. })).count();

    let mut out = String::new();

    match format {
        ExportFormat::PowerShell => {
            out.push_str(&format!(
                "# {line}\n\
                 # kterm layout export — window {window_id}\n\
                 # Generated: {now}\n\
                 # Recreates: {tab_count} tab{ts}, {split_count} split{ss}\n\
                 # {line}\n\n\
                 # Executable path definition\n\
                 $KTERM = if ($env:KTERM) {{ $env:KTERM }} else {{ '{exe_path}' }}\n\n",
                line = "=".repeat(60),
                window_id = window_id,
                now = now,
                tab_count = tab_count,
                ts = if tab_count == 1 { "" } else { "s" },
                split_count = split_count,
                ss = if split_count == 1 { "" } else { "s" },
                exe_path = exe_path.replace('\'', "''"),
            ));

            for step in &steps {
                match step {
                    ActionStep::CreateRoot { tab_id } => {
                        let var = lookup_var(tab_id);
                        if let Some(tab) = find_tab(tab_id) {
                            let is_default_title = tab.title == format!("{} ({})", tab.profile, tab.id);
                            let display_title = if is_default_title {
                                tab.profile.clone()
                            } else {
                                tab.title.clone()
                            };

                            out.push_str(&format!(
                                "# Create {profile} shell for Tab \"{display}\" — assigned to {var}\n\
                                 {var} = (& \"$KTERM\" --profile {profile} --window {window}).Trim()\n\n",
                                profile = tab.profile,
                                display = display_title,
                                var = var,
                                window = window_id,
                            ));
                        }
                    }
                    ActionStep::Split { target_id, direction, new_id } => {
                        let target_var = lookup_var(target_id);
                        let new_var = lookup_var(new_id);
                        let flag = direction_flag_ps1(direction);
                        if let Some(tab) = find_tab(new_id) {
                            let is_default_title = tab.title == format!("{} ({})", tab.profile, tab.id);
                            let display_title = if is_default_title {
                                tab.profile.clone()
                            } else {
                                tab.title.clone()
                            };

                            out.push_str(&format!(
                                "# Split {target_var} {direction} for Tab \"{display}\" — assigned to {new_var}\n\
                                 {new_var} = (& \"$KTERM\" --select-tab \"{target_var}\" {flag} --profile {profile} --window {window}).Trim()\n\n",
                                target_var = target_var,
                                direction = direction,
                                display = display_title,
                                new_var = new_var,
                                flag = flag,
                                profile = tab.profile,
                                window = window_id,
                            ));
                        }
                    }
                }
            }

            // ── Apply metadata ───────────────────────────────────────────────
            let has_meta = ordered_tabs.iter().any(|t| {
                let is_default = t.title == format!("{} ({})", t.profile, t.id);
                !is_default || t.badge.is_some() || t.color.is_some()
            });

            if has_meta {
                out.push_str("# --- Apply tab metadata ---\n\n");
                for tab in &ordered_tabs {
                    let var = lookup_var(&tab.id);
                    let is_default = tab.title == format!("{} ({})", tab.profile, tab.id);

                    if !is_default {
                        out.push_str(&format!(
                            "# Set title for {var}\n\
                             & \"$KTERM\" --select-tab \"{var}\" --send-title {title}\n\n",
                            var = var,
                            title = shell_quote_ps1(&tab.title),
                        ));
                    }

                    if let Some(badge) = &tab.badge {
                        out.push_str(&format!(
                            "# Set badge for {var}\n\
                             & \"$KTERM\" --select-tab \"{var}\" --set-badge {badge}\n\n",
                            var = var,
                            badge = shell_quote_ps1(badge),
                        ));
                    }

                    if let Some(color) = &tab.color {
                        out.push_str(&format!(
                            "# Set colour for {var}\n\
                             & \"$KTERM\" --select-tab \"{var}\" --set-color {color}\n\n",
                            var = var,
                            color = color,
                        ));
                    }
                }
            }

            out.push_str(
                "# ============================================================\n\
                 # kterm Scripting Cheat Sheet & Examples\n\
                 # ============================================================\n\
                 #\n\
                 # 1. Spawning standalone tabs & windows:\n\
                 #    $newTab = (& \"$KTERM\" --profile powershell --window win-1).Trim()\n\
                 #    $newWin = (& \"$KTERM\" --new-window).Trim()\n\
                 #\n\
                 # 2. Splitting panes:\n\
                 #    $rightPane = (& \"$KTERM\" --select-tab \"$tab1\" --split-right --profile wsl).Trim()\n\
                 #    $downPane  = (& \"$KTERM\" --select-tab \"$tab1\" --split-down --profile cmd).Trim()\n\
                 #    $leftPane  = (& \"$KTERM\" --select-tab \"$tab1\" --split-left --profile git-bash).Trim()\n\
                 #    $upPane    = (& \"$KTERM\" --select-tab \"$tab1\" --split-up --profile powershell).Trim()\n\
                 #\n\
                 # 3. Sending text / commands to a tab or pane (unquoted):\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --send-text git status\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --send-text npm run dev\n\
                 #\n\
                 # 4. Customizing tab titles (unquoted), badges, and colors:\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --send-title Server Logs\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --set-badge PROD\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --set-color #E53935\n\
                 #\n\
                 # 5. Window title & focus:\n\
                 #    & \"$KTERM\" --window win-1 --set-window-title Main Workspace\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --focus\n\
                 #\n\
                 # 6. Unsplitting / exploding layout:\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --unsplit\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --explode-split\n\
                 #\n\
                 # 7. Listing active windows & tabs:\n\
                 #    & \"$KTERM\" --list-windows\n\
                 #    & \"$KTERM\" --list-tabs --window win-1 --json\n\
                 #\n\
                 # 8. Closing tabs & windows:\n\
                 #    & \"$KTERM\" --select-tab \"$tab1\" --close --force\n\
                 #    & \"$KTERM\" --close-window win-1\n\
                 #\n\
                 # 9. Exporting window layout to script:\n\
                 #    & \"$KTERM\" --window win-1 --export-script layout.ps1 --format ps1\n\
                 # ============================================================\n",
            );
        }

        ExportFormat::Batch => {
            out.push_str(&format!(
                "@ECHO OFF\n\
                 :: {line}\n\
                 :: kterm layout export - window {window_id}\n\
                 :: Generated: {now}\n\
                 :: Recreates: {tab_count} tab{ts}, {split_count} split{ss}\n\
                 :: {line}\n\n\
                 :: Executable path definition\n\
                 IF NOT DEFINED KTERM SET \"KTERM={exe_path}\"\n\n",
                line = "=".repeat(60),
                window_id = window_id,
                now = now,
                tab_count = tab_count,
                ts = if tab_count == 1 { "" } else { "s" },
                split_count = split_count,
                ss = if split_count == 1 { "" } else { "s" },
                exe_path = exe_path,
            ));

            for step in &steps {
                match step {
                    ActionStep::CreateRoot { tab_id } => {
                        let var = lookup_var(tab_id);
                        if let Some(tab) = find_tab(tab_id) {
                            let is_default_title = tab.title == format!("{} ({})", tab.profile, tab.id);
                            let display_title = if is_default_title {
                                tab.profile.clone()
                            } else {
                                tab.title.clone()
                            };

                            out.push_str(&format!(
                                ":: Create {profile} shell for Tab \"{display}\" — assigned to {var}\n\
                                 FOR /F \"usebackq tokens=*\" %%I IN (`\"%%KTERM%%\" --profile {profile} --window {window}`) DO SET {var}=%%I\n\n",
                                profile = tab.profile,
                                display = display_title,
                                var = var,
                                window = window_id,
                            ));
                        }
                    }
                    ActionStep::Split { target_id, direction, new_id } => {
                        let target_var = lookup_var(target_id);
                        let new_var = lookup_var(new_id);
                        let flag = direction_flag_ps1(direction);
                        if let Some(tab) = find_tab(new_id) {
                            let is_default_title = tab.title == format!("{} ({})", tab.profile, tab.id);
                            let display_title = if is_default_title {
                                tab.profile.clone()
                            } else {
                                tab.title.clone()
                            };

                            out.push_str(&format!(
                                ":: Split %{target_var}% {direction} for Tab \"{display}\" — assigned to %{new_var}%\n\
                                 FOR /F \"usebackq tokens=*\" %%I IN (`\"%%KTERM%%\" --select-tab %{target_var}% {flag} --profile {profile} --window {window}`) DO SET {new_var}=%%I\n\n",
                                target_var = target_var,
                                direction = direction,
                                display = display_title,
                                new_var = new_var,
                                flag = flag,
                                profile = tab.profile,
                                window = window_id,
                            ));
                        }
                    }
                }
            }

            // ── Apply metadata ───────────────────────────────────────────────
            let has_meta = ordered_tabs.iter().any(|t| {
                let is_default = t.title == format!("{} ({})", t.profile, t.id);
                !is_default || t.badge.is_some() || t.color.is_some()
            });

            if has_meta {
                out.push_str(":: --- Apply tab metadata ---\n\n");
                for tab in &ordered_tabs {
                    let var = lookup_var(&tab.id);
                    let is_default = tab.title == format!("{} ({})", tab.profile, tab.id);

                    if !is_default {
                        out.push_str(&format!(
                            ":: Set title for %{var}%\n\
                             \"%%KTERM%%\" --select-tab %{var}% --send-title {title}\n\n",
                            var = var,
                            title = tab.title,
                        ));
                    }

                    if let Some(badge) = &tab.badge {
                        out.push_str(&format!(
                            ":: Set badge for %{var}%\n\
                             \"%%KTERM%%\" --select-tab %{var}% --set-badge {badge}\n\n",
                            var = var,
                            badge = badge,
                        ));
                    }

                    if let Some(color) = &tab.color {
                        out.push_str(&format!(
                            ":: Set colour for %{var}%\n\
                             \"%%KTERM%%\" --select-tab %{var}% --set-color {color}\n\n",
                            var = var,
                            color = color,
                        ));
                    }
                }
            }

            out.push_str(
                ":: ============================================================\n\
                 :: kterm Scripting Cheat Sheet & Examples\n\
                 :: ============================================================\n\
                 ::\n\
                 :: 1. Spawning standalone tabs & windows:\n\
                 ::    FOR /F \"usebackq tokens=*\" %%I IN (`\"%%KTERM%%\" --profile powershell --window win-1`) DO SET NEW_TAB=%%I\n\
                 ::    FOR /F \"usebackq tokens=*\" %%I IN (`\"%%KTERM%%\" --new-window`) DO SET NEW_WIN=%%I\n\
                 ::\n\
                 :: 2. Splitting panes:\n\
                 ::    FOR /F \"usebackq tokens=*\" %%I IN (`\"%%KTERM%%\" --select-tab %%TAB_1%% --split-right --profile wsl`) DO SET PANE_R=%%I\n\
                 ::    FOR /F \"usebackq tokens=*\" %%I IN (`\"%%KTERM%%\" --select-tab %%TAB_1%% --split-down --profile cmd`) DO SET PANE_D=%%I\n\
                 ::    FOR /F \"usebackq tokens=*\" %%I IN (`\"%%KTERM%%\" --select-tab %%TAB_1%% --split-left --profile git-bash`) DO SET PANE_L=%%I\n\
                 ::    FOR /F \"usebackq tokens=*\" %%I IN (`\"%%KTERM%%\" --select-tab %%TAB_1%% --split-up --profile powershell`) DO SET PANE_U=%%I\n\
                 ::\n\
                 :: 3. Sending text / commands to a tab or pane (unquoted):\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --send-text git status\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --send-text npm run dev\n\
                 ::\n\
                 :: 4. Customizing tab titles (unquoted), badges, and colors:\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --send-title Server Logs\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --set-badge PROD\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --set-color #E53935\n\
                 ::\n\
                 :: 5. Window title & focus:\n\
                 ::    \"%%KTERM%%\" --window win-1 --set-window-title Main Workspace\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --focus\n\
                 ::\n\
                 :: 6. Unsplitting / exploding layout:\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --unsplit\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --explode-split\n\
                 ::\n\
                 :: 7. Listing active windows & tabs:\n\
                 ::    \"%%KTERM%%\" --list-windows\n\
                 ::    \"%%KTERM%%\" --list-tabs --window win-1 --json\n\
                 ::\n\
                 :: 8. Closing tabs & windows:\n\
                 ::    \"%%KTERM%%\" --select-tab %%TAB_1%% --close --force\n\
                 ::    \"%%KTERM%%\" --close-window win-1\n\
                 ::\n\
                 :: 9. Exporting window layout to script:\n\
                 ::    \"%%KTERM%%\" --window win-1 --export-script layout.bat --format bat\n\
                 :: ============================================================\n",
            );
        }

        ExportFormat::Shell => {
            let sh_exe_path = exe_path.replace('\\', "/");
            out.push_str(&format!(
                "#!/usr/bin/env bash\n\
                 # {line}\n\
                 # kterm layout export — window {window_id}\n\
                 # Generated: {now}\n\
                 # Recreates: {tab_count} tab{ts}, {split_count} split{ss}\n\
                 # {line}\n\n\
                 # Executable path definition\n\
                 KTERM=\"${{KTERM:-{sh_exe_path}}}\"\n\n",
                line = "=".repeat(60),
                window_id = window_id,
                now = now,
                tab_count = tab_count,
                ts = if tab_count == 1 { "" } else { "s" },
                split_count = split_count,
                ss = if split_count == 1 { "" } else { "s" },
                sh_exe_path = sh_exe_path,
            ));

            for step in &steps {
                match step {
                    ActionStep::CreateRoot { tab_id } => {
                        let var = lookup_var(tab_id);
                        if let Some(tab) = find_tab(tab_id) {
                            let is_default_title = tab.title == format!("{} ({})", tab.profile, tab.id);
                            let display_title = if is_default_title {
                                tab.profile.clone()
                            } else {
                                tab.title.clone()
                            };

                            out.push_str(&format!(
                                "# Create {profile} shell for Tab \"{display}\" — assigned to {var}\n\
                                 {var}=\"$(\"$KTERM\" --profile {profile} --window {window})\"\n\n",
                                profile = tab.profile,
                                display = display_title,
                                var = var,
                                window = window_id,
                            ));
                        }
                    }
                    ActionStep::Split { target_id, direction, new_id } => {
                        let target_var = lookup_var(target_id);
                        let new_var = lookup_var(new_id);
                        let flag = direction_flag_ps1(direction);
                        if let Some(tab) = find_tab(new_id) {
                            let is_default_title = tab.title == format!("{} ({})", tab.profile, tab.id);
                            let display_title = if is_default_title {
                                tab.profile.clone()
                            } else {
                                tab.title.clone()
                            };

                            out.push_str(&format!(
                                "# Split ${target_var} {direction} for Tab \"{display}\" — assigned to {new_var}\n\
                                 {new_var}=\"$(\"$KTERM\" --select-tab \"${target_var}\" {flag} --profile {profile} --window {window})\"\n\n",
                                target_var = target_var,
                                direction = direction,
                                display = display_title,
                                new_var = new_var,
                                flag = flag,
                                profile = tab.profile,
                                window = window_id,
                            ));
                        }
                    }
                }
            }

            // ── Apply metadata ───────────────────────────────────────────────
            let has_meta = ordered_tabs.iter().any(|t| {
                let is_default = t.title == format!("{} ({})", t.profile, t.id);
                !is_default || t.badge.is_some() || t.color.is_some()
            });

            if has_meta {
                out.push_str("# --- Apply tab metadata ---\n\n");
                for tab in &ordered_tabs {
                    let var = lookup_var(&tab.id);
                    let is_default = tab.title == format!("{} ({})", tab.profile, tab.id);

                    if !is_default {
                        out.push_str(&format!(
                            "# Set title for ${var}\n\
                             \"$KTERM\" --select-tab \"${var}\" --send-title {title}\n\n",
                            var = var,
                            title = shell_quote_ps1(&tab.title),
                        ));
                    }

                    if let Some(badge) = &tab.badge {
                        out.push_str(&format!(
                            "# Set badge for ${var}\n\
                             \"$KTERM\" --select-tab \"${var}\" --set-badge {badge}\n\n",
                            var = var,
                            badge = shell_quote_ps1(badge),
                        ));
                    }

                    if let Some(color) = &tab.color {
                        out.push_str(&format!(
                            "# Set colour for ${var}\n\
                             \"$KTERM\" --select-tab \"${var}\" --set-color {color}\n\n",
                            var = var,
                            color = color,
                        ));
                    }
                }
            }

            out.push_str(
                "# ============================================================\n\
                 # kterm Scripting Cheat Sheet & Examples\n\
                 # ============================================================\n\
                 #\n\
                 # 1. Spawning standalone tabs & windows:\n\
                 #    new_tab=\"$(\"$KTERM\" --profile powershell --window win-1)\"\n\
                 #    new_win=\"$(\"$KTERM\" --new-window)\"\n\
                 #\n\
                 # 2. Splitting panes:\n\
                 #    pane_r=\"$(\"$KTERM\" --select-tab \"$tab1\" --split-right --profile wsl)\"\n\
                 #    pane_d=\"$(\"$KTERM\" --select-tab \"$tab1\" --split-down --profile cmd)\"\n\
                 #    pane_l=\"$(\"$KTERM\" --select-tab \"$tab1\" --split-left --profile git-bash)\"\n\
                 #    pane_u=\"$(\"$KTERM\" --select-tab \"$tab1\" --split-up --profile powershell)\"\n\
                 #\n\
                 # 3. Sending text / commands to a tab or pane (unquoted):\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --send-text git status\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --send-text npm run dev\n\
                 #\n\
                 # 4. Customizing tab titles (unquoted), badges, and colors:\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --send-title Server Logs\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --set-badge PROD\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --set-color #E53935\n\
                 #\n\
                 # 5. Window title & focus:\n\
                 #    \"$KTERM\" --window win-1 --set-window-title Main Workspace\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --focus\n\
                 #\n\
                 # 6. Unsplitting / exploding layout:\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --unsplit\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --explode-split\n\
                 #\n\
                 # 7. Listing active windows & tabs:\n\
                 #    \"$KTERM\" --list-windows\n\
                 #    \"$KTERM\" --list-tabs --window win-1 --json\n\
                 #\n\
                 # 8. Closing tabs & windows:\n\
                 #    \"$KTERM\" --select-tab \"$tab1\" --close --force\n\
                 #    \"$KTERM\" --close-window win-1\n\
                 #\n\
                 # 9. Exporting window layout to script:\n\
                 #    \"$KTERM\" --window win-1 --export-script layout.sh --format sh\n\
                 # ============================================================\n",
            );
        }
    }

    out
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Walk layout tree depth-first to collect split action steps.
fn collect_node_steps(
    node: &LayoutNode,
    steps: &mut Vec<ActionStep>,
    ordered_tab_ids: &mut Vec<String>,
) {
    if let LayoutNode::Split { direction, first, second, .. } = node {
        collect_node_steps(first, steps, ordered_tab_ids);

        let target_id = first_tab_id(first);
        let new_id = first_tab_id(second);
        let dir_str = match direction {
            crate::pty::SplitDirection::Horizontal => "right",
            crate::pty::SplitDirection::Vertical => "down",
        }
        .to_string();

        if !ordered_tab_ids.contains(&new_id) {
            ordered_tab_ids.push(new_id.clone());
        }

        steps.push(ActionStep::Split {
            target_id,
            direction: dir_str,
            new_id,
        });

        collect_node_steps(second, steps, ordered_tab_ids);
    }
}

/// Return the leftmost/topmost tab ID in a layout subtree.
fn first_tab_id(node: &LayoutNode) -> String {
    match node {
        LayoutNode::Pane { tab_id } => tab_id.clone(),
        LayoutNode::Split { first, .. } => first_tab_id(first),
    }
}

/// Generate a variable name for the given 1-based tab index.
fn var_name(n: usize, format: ExportFormat) -> String {
    match format {
        ExportFormat::PowerShell => format!("$tab{}", n),
        ExportFormat::Batch => format!("TAB_{}", n),
        ExportFormat::Shell => format!("tab{}", n),
    }
}

/// Map a direction string to the kterm CLI flag.
fn direction_flag_ps1(dir: &str) -> &'static str {
    match dir {
        "left" => "--split-left",
        "up" => "--split-up",
        "down" => "--split-down",
        _ => "--split-right",
    }
}

/// Wrap a string in single-quotes for PS1 if it contains spaces.
fn shell_quote_ps1(s: &str) -> String {
    if s.contains(' ') {
        format!("'{}'", s.replace('\'', "''"))
    } else {
        s.to_string()
    }
}

/// Current date-time as a human-readable string without pulling in chrono.
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;

    let mut year = 1970u32;
    let mut remaining_days = days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }
    let months = [31u64, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for &mlen in &months {
        if remaining_days < mlen {
            break;
        }
        remaining_days -= mlen;
        month += 1;
    }
    let day = remaining_days + 1;

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", year, month, day, h, m, s)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
