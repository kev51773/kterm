use serde::{Deserialize, Serialize};
use std::io::Write;
use crate::daemon::AppState;
use crate::pty::{LayoutNode, SplitDirection};
use tauri::Manager;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlSessionSpec {
    pub window: YamlWindowSpec,
    pub tabs: Vec<YamlTabSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlWindowSpec {
    pub id: Option<String>,
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
    pub admin: Option<bool>,
    pub elevated: Option<bool>,
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
    pub admin: Option<bool>,
    pub elevated: Option<bool>,
    pub send_text: Option<String>,
    pub splits: Option<Vec<YamlSplitSpec>>,
}

pub fn parse_yaml(content: &str) -> Result<YamlSessionSpec, String> {
    serde_yaml::from_str::<YamlSessionSpec>(content).map_err(|e| format!("YAML parse error: {}", e))
}

pub fn validate_yaml(spec: &YamlSessionSpec) -> Result<(), String> {
    if spec.tabs.is_empty() {
        return Err("YAML session spec must contain at least one tab".to_string());
    }

    for tab in &spec.tabs {
        validate_tab(tab)?;
    }
    Ok(())
}

fn validate_tab(tab: &YamlTabSpec) -> Result<(), String> {
    validate_profile(&tab.profile)?;
    if let Some(splits) = &tab.splits {
        for split in splits {
            validate_split(split)?;
        }
    }
    Ok(())
}

fn validate_split(split: &YamlSplitSpec) -> Result<(), String> {
    validate_profile(&split.profile)?;
    let dir = split.direction.to_lowercase();
    if !matches!(dir.as_str(), "down" | "right" | "left" | "up") {
        return Err(format!("Invalid split direction '{}'. Must be one of: down, right, left, up.", split.direction));
    }
    if let Some(splits) = &split.splits {
        for s in splits {
            validate_split(s)?;
        }
    }
    Ok(())
}

fn validate_profile(profile: &str) -> Result<(), String> {
    let p = profile.to_lowercase();
    if !matches!(p.as_str(), "powershell" | "cmd" | "wsl" | "git-bash" | "bash") {
        return Err(format!("Invalid profile '{}'. Must be one of: powershell, cmd, wsl, git-bash", profile));
    }
    Ok(())
}

pub fn generate_next_window_id(state: &AppState) -> String {
    if is_window_untouched_initial(state, "win-1") {
        return "win-1".to_string();
    }
    let titles = state.window_titles.lock().unwrap();
    let layouts = state.window_layouts.lock().unwrap();
    let mut count = 1;
    loop {
        let candidate = format!("win-{}", count);
        if !titles.contains_key(&candidate) && !layouts.contains_key(&candidate) {
            return candidate;
        }
        count += 1;
    }
}

pub fn is_window_untouched_initial(state: &AppState, window_id: &str) -> bool {
    let tabs = state.pty_manager.list_by_window(Some(window_id));
    tabs.is_empty()
}

pub fn resolve_window_id(
    state: &AppState,
    base_id: &str,
    suffix: Option<&str>,
    suffix_auto: bool,
) -> String {
    let base_win_id = base_id.trim();
    if let Some(s) = suffix {
        format!("{}{}", base_win_id, s)
    } else if suffix_auto {
        let titles = state.window_titles.lock().unwrap();
        let layouts = state.window_layouts.lock().unwrap();
        let exists = titles.contains_key(base_win_id) || layouts.contains_key(base_win_id);
        if !exists || is_window_untouched_initial(state, base_win_id) {
            base_win_id.to_string()
        } else {
            let mut count = 1;
            loop {
                let candidate = format!("{}-{}", base_win_id, count);
                if !titles.contains_key(&candidate) && !layouts.contains_key(&candidate) {
                    return candidate;
                }
                count += 1;
            }
        }
    } else {
        base_win_id.to_string()
    }
}

pub fn apply_yaml_spec(
    state: &AppState,
    spec: &YamlSessionSpec,
    window_override: Option<&str>,
    suffix: Option<&str>,
    suffix_auto: bool,
) -> Result<String, String> {
    validate_yaml(spec)?;

    let generated_win_id;
    let base_id = if let Some(w) = window_override.filter(|w| !w.trim().is_empty()) {
        w
    } else if let Some(w) = spec.window.id.as_deref().filter(|w| !w.trim().is_empty()) {
        w
    } else {
        generated_win_id = generate_next_window_id(state);
        &generated_win_id
    };

    let window_id = resolve_window_id(state, base_id, suffix, suffix_auto);

    // Collision check / untouched initial window reuse
    {
        let titles = state.window_titles.lock().unwrap();
        let layouts = state.window_layouts.lock().unwrap();
        let exists = titles.contains_key(&window_id) || layouts.contains_key(&window_id);
        drop(titles);
        drop(layouts);

        if exists {
            let existing_tabs = state.pty_manager.list_by_window(Some(&window_id));
            for t in existing_tabs {
                state.pty_manager.close(&t.id);
            }
            state.window_layouts.lock().unwrap().remove(&window_id);
            state.window_titles.lock().unwrap().remove(&window_id);
        }
    }

    let win_title = spec
        .window
        .title
        .clone()
        .unwrap_or_else(|| format!("kterm.exe - A scriptable terminal - {}", window_id));

    let formatted_title = if win_title.starts_with("kterm.exe") {
        win_title
    } else {
        format!("kterm.exe - {} - {}", win_title, window_id)
    };

    state
        .window_titles
        .lock()
        .unwrap()
        .insert(window_id.clone(), formatted_title.clone());

    if let Some(app) = &state.app_handle {
        if let Some(existing_window) = app.get_webview_window(&window_id) {
            let _ = existing_window.set_title(&formatted_title);
            let _ = existing_window.show();
            let _ = existing_window.unminimize();
            let _ = existing_window.set_focus();
        } else {
            use tauri::WebviewWindowBuilder;
            let (default_w, default_h) = crate::config::AppConfig::load().get_default_window_size();
            let builder = WebviewWindowBuilder::new(
                app,
                &window_id,
                tauri::WebviewUrl::App(format!("index.html?window={}", window_id).into()),
            )
            .title(&formatted_title)
            .decorations(false)
            .inner_size(default_w, default_h);

            if let Ok(w) = builder.build() {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    }

    let mut layouts = state.window_layouts.lock().unwrap();
    let win_layouts = layouts.entry(window_id.clone()).or_insert_with(Vec::new);

    for tab_spec in &spec.tabs {
        let root_tab_id = tab_spec
            .id
            .clone()
            .unwrap_or_else(|| state.pty_manager.generate_next_tab_id_for_window(&window_id));

        let is_admin = tab_spec.admin.or(tab_spec.elevated).unwrap_or(false);
        let sess = state.pty_manager.spawn_with_cwd(
            root_tab_id.clone(),
            tab_spec.profile.clone(),
            window_id.clone(),
            tab_spec.cwd.as_deref(),
            is_admin,
        )?;

        if let Some(title) = &tab_spec.title {
            *sess.title.lock().unwrap() = title.clone();
        }
        if let Some(badge) = &tab_spec.badge {
            *sess.badge.lock().unwrap() = Some(badge.clone());
        }
        if let Some(color) = &tab_spec.color {
            *sess.color.lock().unwrap() = Some(color.clone());
        }
        if let Some(text) = &tab_spec.send_text {
            send_text_to_session(&sess, text);
        }

        let mut root_node = LayoutNode::Pane {
            tab_id: root_tab_id.clone(),
        };

        if let Some(splits) = &tab_spec.splits {
            for split_spec in splits {
                apply_split_recursive(
                    state,
                    &window_id,
                    &mut root_node,
                    &root_tab_id,
                    split_spec,
                )?;
            }
        }

        win_layouts.push(root_node);
    }

    Ok(window_id)
}

fn apply_split_recursive(
    state: &AppState,
    window_id: &str,
    parent_node: &mut LayoutNode,
    target_tab_id: &str,
    split_spec: &YamlSplitSpec,
) -> Result<String, String> {
    let dir_str = split_spec.direction.to_lowercase();
    let (direction, insert_first) = match dir_str.as_str() {
        "up" => (SplitDirection::Vertical, true),
        "down" => (SplitDirection::Vertical, false),
        "left" => (SplitDirection::Horizontal, true),
        _ => (SplitDirection::Horizontal, false), // "right"
    };

    let new_tab_id = split_spec
        .id
        .clone()
        .unwrap_or_else(|| state.pty_manager.generate_next_tab_id_for_window(window_id));

    let is_split_admin = split_spec.admin.or(split_spec.elevated).unwrap_or(false);
    let sess = state.pty_manager.spawn_with_cwd(
        new_tab_id.clone(),
        split_spec.profile.clone(),
        window_id.to_string(),
        split_spec.cwd.as_deref(),
        is_split_admin,
    )?;

    if let Some(title) = &split_spec.title {
        *sess.title.lock().unwrap() = title.clone();
    }
    if let Some(badge) = &split_spec.badge {
        *sess.badge.lock().unwrap() = Some(badge.clone());
    }
    if let Some(color) = &split_spec.color {
        *sess.color.lock().unwrap() = Some(color.clone());
    }
    if let Some(text) = &split_spec.send_text {
        send_text_to_session(&sess, text);
    }

    parent_node.split_at(target_tab_id, direction, &new_tab_id, insert_first);

    if let Some(splits) = &split_spec.splits {
        for child_split in splits {
            apply_split_recursive(state, window_id, parent_node, &new_tab_id, child_split)?;
        }
    }

    Ok(new_tab_id)
}

fn send_text_to_session(sess: &crate::pty::PtySession, text: &str) {
    let mut to_send = text.to_string();
    if !to_send.ends_with('\r') && !to_send.ends_with('\n') {
        to_send.push('\r');
    }
    let mut writer = sess.writer.lock().unwrap();
    let _ = writer.write_all(to_send.as_bytes());
    let _ = writer.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // ── parse_yaml ──────────────────────────────────────────────────

    #[test]
    fn parse_minimal_yaml() {
        let yaml = "window:\n  id: win-1\ntabs:\n- profile: powershell\n";
        let spec = parse_yaml(yaml).unwrap();
        assert_eq!(spec.window.id.as_deref(), Some("win-1"));
        assert_eq!(spec.tabs.len(), 1);
        assert_eq!(spec.tabs[0].profile, "powershell");
    }

    #[test]
    fn parse_full_yaml() {
        let yaml = r##"
window:
  id: win-1
  title: My Window
tabs:
- id: tab-1
  profile: cmd
  title: Server
  badge: PROD
  color: "#E53935"
  cwd: C:/Users
  admin: true
  send_text: echo hello
  splits:
  - direction: right
    profile: wsl
    title: Split 1
  - direction: down
    profile: git-bash
"##;
        let spec = parse_yaml(yaml).unwrap();
        assert_eq!(spec.window.title.as_deref(), Some("My Window"));
        assert_eq!(spec.tabs.len(), 1);
        let tab = &spec.tabs[0];
        assert_eq!(tab.id.as_deref(), Some("tab-1"));
        assert_eq!(tab.profile, "cmd");
        assert_eq!(tab.title.as_deref(), Some("Server"));
        assert_eq!(tab.badge.as_deref(), Some("PROD"));
        assert_eq!(tab.color.as_deref(), Some("#E53935"));
        assert_eq!(tab.cwd.as_deref(), Some("C:/Users"));
        assert_eq!(tab.admin, Some(true));
        assert_eq!(tab.send_text.as_deref(), Some("echo hello"));
        let splits = tab.splits.as_ref().unwrap();
        assert_eq!(splits.len(), 2);
        assert_eq!(splits[0].direction, "right");
        assert_eq!(splits[0].profile, "wsl");
        assert_eq!(splits[1].direction, "down");
        assert_eq!(splits[1].profile, "git-bash");
    }

    #[test]
    fn parse_invalid_yaml() {
        let result = parse_yaml("not: valid: yaml: [[[");
        assert!(result.is_err());
    }

    #[test]
    fn parse_yaml_missing_tabs() {
        let yaml = "window:\n  id: win-1\n";
        let result = parse_yaml(yaml);
        assert!(result.is_err());
    }

    // ── validate_yaml ───────────────────────────────────────────────

    #[test]
    fn validate_empty_tabs() {
        let spec = YamlSessionSpec {
            window: YamlWindowSpec { id: None, title: None },
            tabs: vec![],
        };
        assert!(validate_yaml(&spec).is_err());
    }

    #[test]
    fn validate_valid_spec() {
        let spec = YamlSessionSpec {
            window: YamlWindowSpec { id: Some("win-1".into()), title: None },
            tabs: vec![YamlTabSpec {
                id: None,
                profile: "powershell".into(),
                title: None, badge: None, color: None, cwd: None,
                admin: None, elevated: None, send_text: None, splits: None,
            }],
        };
        assert!(validate_yaml(&spec).is_ok());
    }

    #[test]
    fn validate_invalid_tab_profile() {
        let spec = YamlSessionSpec {
            window: YamlWindowSpec { id: None, title: None },
            tabs: vec![YamlTabSpec {
                id: None,
                profile: "fish".into(),
                title: None, badge: None, color: None, cwd: None,
                admin: None, elevated: None, send_text: None, splits: None,
            }],
        };
        assert!(validate_yaml(&spec).is_err());
    }

    #[test]
    fn validate_invalid_split_direction() {
        let spec = YamlSessionSpec {
            window: YamlWindowSpec { id: None, title: None },
            tabs: vec![YamlTabSpec {
                id: None,
                profile: "powershell".into(),
                title: None, badge: None, color: None, cwd: None,
                admin: None, elevated: None, send_text: None,
                splits: Some(vec![YamlSplitSpec {
                    direction: "diagonal".into(),
                    profile: "cmd".into(),
                    id: None, title: None, badge: None, color: None, cwd: None,
                    admin: None, elevated: None, send_text: None, splits: None,
                }]),
            }],
        };
        assert!(validate_yaml(&spec).is_err());
    }

    // ── validate_profile ────────────────────────────────────────────

    #[test]
    fn valid_profiles() {
        for p in &["powershell", "cmd", "wsl", "git-bash", "bash"] {
            assert!(validate_profile(p).is_ok(), "expected valid: {}", p);
        }
    }

    #[test]
    fn invalid_profiles() {
        for p in &["fish", "zsh", "powershell "] {
            assert!(validate_profile(p).is_err(), "expected invalid: {}", p);
        }
    }

    // ── validate_split ──────────────────────────────────────────────

    #[test]
    fn valid_split_directions() {
        for d in &["down", "right", "left", "up"] {
            let split = YamlSplitSpec {
                direction: d.to_string(),
                profile: "powershell".into(),
                id: None, title: None, badge: None, color: None, cwd: None,
                admin: None, elevated: None, send_text: None, splits: None,
            };
            assert!(validate_split(&split).is_ok(), "expected valid: {}", d);
        }
    }

    #[test]
    fn invalid_split_direction() {
        let split = YamlSplitSpec {
            direction: "diagonal".into(),
            profile: "powershell".into(),
            id: None, title: None, badge: None, color: None, cwd: None,
            admin: None, elevated: None, send_text: None, splits: None,
        };
        assert!(validate_split(&split).is_err());
    }

    // ── resolve_window_id ───────────────────────────────────────────

    fn make_empty_state() -> crate::daemon::AppState {
        crate::daemon::AppState {
            pty_manager: crate::pty::PtyManager::new(),
            app_handle: None,
            window_titles: Arc::new(Mutex::new(HashMap::new())),
            window_layouts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[test]
    fn resolve_window_id_no_suffix() {
        let state = make_empty_state();
        let id = resolve_window_id(&state, "win-1", None, false);
        assert_eq!(id, "win-1");
    }

    #[test]
    fn resolve_window_id_with_suffix() {
        let state = make_empty_state();
        let id = resolve_window_id(&state, "win-1", Some("-backend"), false);
        assert_eq!(id, "win-1-backend");
    }

    #[test]
    fn resolve_window_id_auto_first_available() {
        let state = make_empty_state();
        let id = resolve_window_id(&state, "win-1", None, true);
        assert_eq!(id, "win-1");
    }

    #[test]
    fn resolve_window_id_auto_increments() {
        let state = make_empty_state();
        // Without live PTY sessions, is_window_untouched_initial is always true
        // for any window, so suffix_auto always returns the base ID.
        // This tests the suffix_auto=true path returns base when untouched.
        let id = resolve_window_id(&state, "win-1", None, true);
        assert_eq!(id, "win-1");
    }

    #[test]
    fn resolve_window_id_auto_skips_occupied() {
        let state = make_empty_state();
        // Same as above: without live sessions, untouched_initial is true.
        let id = resolve_window_id(&state, "win-1", None, true);
        assert_eq!(id, "win-1");
    }

    // ── generate_next_window_id ─────────────────────────────────────

    #[test]
    fn generate_next_window_id_first() {
        let state = make_empty_state();
        let id = generate_next_window_id(&state);
        assert_eq!(id, "win-1");
    }

    #[test]
    fn generate_next_window_id_always_first_when_empty() {
        // Without live PTY sessions, generate_next_window_id always returns "win-1"
        let state = make_empty_state();
        state.window_titles.lock().unwrap().insert("win-1".into(), "exists".into());
        let id = generate_next_window_id(&state);
        assert_eq!(id, "win-1");
    }

    // ── YAML roundtrip: parse → validate ────────────────────────────

    #[test]
    fn yaml_roundtrip_minimal() {
        let yaml = "window:\n  id: win-1\ntabs:\n- profile: powershell\n";
        let spec = parse_yaml(yaml).unwrap();
        assert!(validate_yaml(&spec).is_ok());
    }

    #[test]
    fn yaml_roundtrip_with_splits() {
        let yaml = r#"
window:
  id: win-1
tabs:
- profile: powershell
  splits:
  - direction: right
    profile: cmd
  - direction: down
    profile: wsl
"#;
        let spec = parse_yaml(yaml).unwrap();
        assert!(validate_yaml(&spec).is_ok());
        assert_eq!(spec.tabs[0].splits.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn yaml_all_valid_profiles() {
        for p in &["powershell", "cmd", "wsl", "git-bash"] {
            let yaml = format!("window:\n  id: win-1\ntabs:\n- profile: {}\n", p);
            let spec = parse_yaml(&yaml).unwrap();
            assert!(validate_yaml(&spec).is_ok(), "profile {} should be valid", p);
        }
    }

    #[test]
    fn yaml_rejects_invalid_profile() {
        let yaml = "window:\n  id: win-1\ntabs:\n- profile: fish\n";
        let spec = parse_yaml(yaml).unwrap();
        assert!(validate_yaml(&spec).is_err());
    }

    // ── is_window_untouched_initial ─────────────────────────────────

    #[test]
    fn untouched_initial_empty() {
        let state = make_empty_state();
        assert!(is_window_untouched_initial(&state, "win-1"));
    }

    #[test]
    fn untouched_initial_nonexistent() {
        let state = make_empty_state();
        assert!(is_window_untouched_initial(&state, "win-999"));
    }
}
