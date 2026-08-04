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

/// Build and return the script text for the given window.
///
/// Walks the layout tree depth-first (first/left before second/right) to
/// determine tab creation and split order, then appends metadata commands
/// (title, badge, colour) for any tabs that have them set.
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

    // ── 2. Determine DFS tab order from layout tree ──────────────────────────
    let layouts = state.window_layouts.lock().unwrap();
    let win_layouts = match layouts.get(window_id) {
        Some(l) => l.clone(),
        None => vec![],
    };
    drop(layouts);

    // Collect tabs in DFS order across all layout nodes.
    let mut ordered_tab_ids: Vec<String> = Vec::new();
    for node in &win_layouts {
        collect_dfs(node, &mut ordered_tab_ids);
    }
    for t in &tab_infos {
        if !ordered_tab_ids.contains(&t.id) {
            ordered_tab_ids.push(t.id.clone());
        }
    }

    // Build the list of TabInfo in DFS order.
    let ordered_tabs: Vec<&TabInfo> = ordered_tab_ids
        .iter()
        .filter_map(|id| tab_infos.iter().find(|t| &t.id == id))
        .collect();

    // ── 3. Build split operations from the layout tree ───────────────────────
    let mut split_ops: Vec<(String, String, String)> = Vec::new();
    for node in &win_layouts {
        collect_splits(node, &mut split_ops);
    }

    // ── 4. Assign variable names & current exe path ──────────────────────────
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

    // ── 5. Render script ─────────────────────────────────────────────────────
    let now = chrono_now();
    let tab_count = ordered_tabs.len();
    let split_count = split_ops.len();

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

            // ── Create tabs ──────────────────────────────────────────────────
            for (i, tab) in ordered_tabs.iter().enumerate() {
                let var = lookup_var(&tab.id);
                let is_default_title = tab.title == format!("{} ({})", tab.profile, tab.id);
                let display_title = if is_default_title {
                    tab.profile.clone()
                } else {
                    tab.title.clone()
                };

                out.push_str(&format!(
                    "# Create {} shell for Tab \"{display}\" — assigned to {var}\n\
                     {var} = (& \"$KTERM\" --profile {profile} --window {window}).Trim()\n\n",
                    tab.profile,
                    display = display_title,
                    var = var,
                    profile = tab.profile,
                    window = window_id,
                ));

                let _ = i;
            }

            // ── Reconstruct splits ───────────────────────────────────────────
            if !split_ops.is_empty() {
                out.push_str("# --- Reconstruct split layout ---\n\n");
                for (target_id, direction, new_id) in &split_ops {
                    let target_var = lookup_var(target_id);
                    let new_var = lookup_var(new_id);
                    let flag = direction_flag_ps1(direction);
                    out.push_str(&format!(
                        "# Split {target_var} {direction} to place {new_var} beside it\n\
                         & \"$KTERM\" --select-tab \"{target_var}\" {flag} --move-tab \"{new_var}\"\n\n",
                        target_var = target_var,
                        direction = direction,
                        new_var = new_var,
                        flag = flag,
                    ));
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

            // ── Create tabs ──────────────────────────────────────────────────
            for tab in &ordered_tabs {
                let var = lookup_var(&tab.id);
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

            // ── Reconstruct splits ───────────────────────────────────────────
            if !split_ops.is_empty() {
                out.push_str(":: --- Reconstruct split layout ---\n\n");
                for (target_id, direction, new_id) in &split_ops {
                    let target_var = lookup_var(target_id);
                    let new_var = lookup_var(new_id);
                    let flag = direction_flag_ps1(direction);
                    out.push_str(&format!(
                        ":: Split %{target_var}% {direction} to place %{new_var}% beside it\n\
                         \"%%KTERM%%\" --select-tab %{target_var}% {flag} --move-tab %{new_var}%\n\n",
                        target_var = target_var,
                        direction = direction,
                        new_var = new_var,
                        flag = flag,
                    ));
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

            // ── Create tabs ──────────────────────────────────────────────────
            for tab in &ordered_tabs {
                let var = lookup_var(&tab.id);
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

            // ── Reconstruct splits ───────────────────────────────────────────
            if !split_ops.is_empty() {
                out.push_str("# --- Reconstruct split layout ---\n\n");
                for (target_id, direction, new_id) in &split_ops {
                    let target_var = lookup_var(target_id);
                    let new_var = lookup_var(new_id);
                    let flag = direction_flag_ps1(direction);
                    out.push_str(&format!(
                        "# Split ${target_var} {direction} to place ${new_var} beside it\n\
                         \"$KTERM\" --select-tab \"${target_var}\" {flag} --move-tab \"${new_var}\"\n\n",
                        target_var = target_var,
                        direction = direction,
                        new_var = new_var,
                        flag = flag,
                    ));
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
        }
    }

    out
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Walk layout tree depth-first, appending tab IDs in first→second order.
fn collect_dfs(node: &LayoutNode, out: &mut Vec<String>) {
    match node {
        LayoutNode::Pane { tab_id } => {
            if !out.contains(tab_id) {
                out.push(tab_id.clone());
            }
        }
        LayoutNode::Split { first, second, .. } => {
            collect_dfs(first, out);
            collect_dfs(second, out);
        }
    }
}

/// Walk layout tree and collect (target_id, direction_str, new_id) for every
/// Split node encountered depth-first (inner pairs before outer).
fn collect_splits(node: &LayoutNode, out: &mut Vec<(String, String, String)>) {
    if let LayoutNode::Split {
        direction,
        first,
        second,
        ..
    } = node
    {
        // Recurse into children first (inner splits before outer).
        collect_splits(first, out);
        collect_splits(second, out);

        // The "first" pane is the anchor; "second" is what gets split in.
        // We express this as: split at first → place second beside it.
        let target_id = first_tab_id(first);
        let new_id = first_tab_id(second);
        let dir_str = match direction {
            crate::pty::SplitDirection::Horizontal => "right",
            crate::pty::SplitDirection::Vertical => "down",
        }
        .to_string();

        out.push((target_id, dir_str, new_id));
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
    // Use SystemTime — no external dep needed for a display-only timestamp.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Rough UTC breakdown (good enough for a script comment, no DST logic).
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400; // days since 1970-01-01

    // Gregorian calendar approximation.
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
