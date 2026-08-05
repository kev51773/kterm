use crate::daemon::AppState;
use crate::pty::LayoutNode;
use crate::yaml::{YamlSessionSpec, YamlSplitSpec, YamlTabSpec, YamlWindowSpec};
use std::sync::Arc;
use crate::pty::PtySession;

pub fn export_yaml_layout(state: &AppState, window_id: &str) -> String {
    let sessions = state.pty_manager.list_by_window(Some(window_id));
    let titles_map = state.window_titles.lock().unwrap();
    let win_title = titles_map.get(window_id).cloned();
    drop(titles_map);

    let layouts = state.window_layouts.lock().unwrap();
    let win_layouts = match layouts.get(window_id) {
        Some(l) => l.clone(),
        None => vec![],
    };
    drop(layouts);

    let mut tabs: Vec<YamlTabSpec> = Vec::new();
    let mut processed_tab_ids: Vec<String> = Vec::new();

    for node in &win_layouts {
        if let Some(tab_spec) = convert_node_to_tab(node, &sessions, &mut processed_tab_ids) {
            tabs.push(tab_spec);
        }
    }

    for sess in &sessions {
        if !processed_tab_ids.contains(&sess.id) {
            processed_tab_ids.push(sess.id.clone());
            let default_title = format!("{} ({})", sess.profile, sess.id);
            let cur_title = sess.title.lock().unwrap().clone();
            let title = if cur_title != default_title {
                Some(cur_title)
            } else {
                None
            };
            tabs.push(YamlTabSpec {
                id: Some(sess.id.clone()),
                profile: sess.profile.clone(),
                title,
                badge: sess.badge.lock().unwrap().clone(),
                color: sess.color.lock().unwrap().clone(),
                cwd: None,
                send_text: None,
                splits: None,
            });
        }
    }

    let spec = YamlSessionSpec {
        window: YamlWindowSpec {
            id: window_id.to_string(),
            title: win_title,
        },
        tabs,
    };

    serde_yaml::to_string(&spec).unwrap_or_else(|e| format!("# Error serializing layout: {}", e))
}

fn convert_node_to_tab(
    node: &LayoutNode,
    sessions: &[Arc<PtySession>],
    processed_tab_ids: &mut Vec<String>,
) -> Option<YamlTabSpec> {
    match node {
        LayoutNode::Pane { tab_id } => {
            if !processed_tab_ids.contains(tab_id) {
                processed_tab_ids.push(tab_id.clone());
            }
            let sess = sessions.iter().find(|s| &s.id == tab_id)?;
            let default_title = format!("{} ({})", sess.profile, sess.id);
            let cur_title = sess.title.lock().unwrap().clone();
            let title = if cur_title != default_title {
                Some(cur_title)
            } else {
                None
            };

            Some(YamlTabSpec {
                id: Some(sess.id.clone()),
                profile: sess.profile.clone(),
                title,
                badge: sess.badge.lock().unwrap().clone(),
                color: sess.color.lock().unwrap().clone(),
                cwd: None,
                send_text: None,
                splits: None,
            })
        }
        LayoutNode::Split {
            direction,
            first,
            second,
            ..
        } => {
            let mut base_tab = convert_node_to_tab(first, sessions, processed_tab_ids)?;
            let dir_str = match direction {
                crate::pty::SplitDirection::Horizontal => "right",
                crate::pty::SplitDirection::Vertical => "down",
            }
            .to_string();

            let split_spec = convert_node_to_split(second, dir_str, sessions, processed_tab_ids)?;

            let splits = base_tab.splits.get_or_insert_with(Vec::new);
            splits.push(split_spec);

            Some(base_tab)
        }
    }
}

fn convert_node_to_split(
    node: &LayoutNode,
    dir: String,
    sessions: &[Arc<PtySession>],
    processed_tab_ids: &mut Vec<String>,
) -> Option<YamlSplitSpec> {
    match node {
        LayoutNode::Pane { tab_id } => {
            if !processed_tab_ids.contains(tab_id) {
                processed_tab_ids.push(tab_id.clone());
            }
            let sess = sessions.iter().find(|s| &s.id == tab_id)?;
            let default_title = format!("{} ({})", sess.profile, sess.id);
            let cur_title = sess.title.lock().unwrap().clone();
            let title = if cur_title != default_title {
                Some(cur_title)
            } else {
                None
            };

            Some(YamlSplitSpec {
                direction: dir,
                profile: sess.profile.clone(),
                id: Some(sess.id.clone()),
                title,
                badge: sess.badge.lock().unwrap().clone(),
                color: sess.color.lock().unwrap().clone(),
                cwd: None,
                send_text: None,
                splits: None,
            })
        }
        LayoutNode::Split {
            direction,
            first,
            second,
            ..
        } => {
            let mut base_split = convert_node_to_split(first, dir, sessions, processed_tab_ids)?;
            let child_dir = match direction {
                crate::pty::SplitDirection::Horizontal => "right",
                crate::pty::SplitDirection::Vertical => "down",
            }
            .to_string();

            let child_split = convert_node_to_split(second, child_dir, sessions, processed_tab_ids)?;
            let splits = base_split.splits.get_or_insert_with(Vec::new);
            splits.push(child_split);

            Some(base_split)
        }
    }
}

pub fn export_shortcut_for_yaml(yaml_path_str: &str) -> Result<String, String> {
    let yaml_path = std::path::Path::new(yaml_path_str);
    let abs_yaml = if yaml_path.is_absolute() {
        yaml_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current dir: {}", e))?
            .join(yaml_path)
    };

    let shortcut_path = abs_yaml.with_extension("lnk");

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get current exe path: {}", e))?;

    let exe_str = exe_path.to_string_lossy().to_string();
    let yaml_str = abs_yaml.to_string_lossy().to_string();
    let shortcut_str = shortcut_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        let ps_script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $s = $ws.CreateShortcut('{}'); \
             $s.TargetPath = '{}'; \
             $s.Arguments = '--apply \"{}\" --suffix-auto'; \
             $s.Save()",
            shortcut_str.replace('\'', "''"),
            exe_str.replace('\'', "''"),
            yaml_str.replace('\'', "''")
        );

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
            .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("PowerShell shortcut creation failed: {}", stderr));
        }
    }

    Ok(shortcut_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_shortcut_creation() {
        let temp_dir = std::env::temp_dir();
        let test_yaml = temp_dir.join("test_layout_shortcut_unit.yaml");
        std::fs::write(&test_yaml, "window:\n  id: win-1\n").unwrap();

        let shortcut_res = export_shortcut_for_yaml(test_yaml.to_str().unwrap());
        assert!(shortcut_res.is_ok(), "Shortcut creation failed: {:?}", shortcut_res.err());

        let shortcut_path = std::path::PathBuf::from(shortcut_res.unwrap());
        assert!(shortcut_path.exists(), "Shortcut file does not exist");
        assert_eq!(shortcut_path.extension().unwrap(), "lnk");

        let _ = std::fs::remove_file(&test_yaml);
        let _ = std::fs::remove_file(&shortcut_path);
    }
}

