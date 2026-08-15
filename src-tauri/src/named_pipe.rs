use crate::daemon::{
    ApplySessionRequest, AppState, CreateTabRequest, ReadTabQuery, SendTextRequest, SetBadgeRequest,
    SetColorRequest, SetTitleRequest, TargetTabRequest, WaitTabReq,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const PIPE_NAME: &str = r"\\.\pipe\kterm_daemon";

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CliRequest {
    HealthCheck,
    ListTabs {
        window: Option<String>,
    },
    SpawnTab {
        profile: Option<String>,
        window: Option<String>,
        admin: Option<bool>,
        elevated: Option<bool>,
        cwd: Option<String>,
        cols: Option<u16>,
        rows: Option<u16>,
    },
    CloseTabs {
        targets: Vec<String>,
        window: Option<String>,
        force: Option<bool>,
    },
    SplitTab {
        tab_id: String,
        direction: String,
        profile: Option<String>,
        move_tab_id: Option<String>,
    },
    UnsplitTab {
        tab_id: String,
    },
    ExplodeTab {
        tab_id: String,
    },
    SetTitle {
        targets: Vec<String>,
        title: String,
        window: Option<String>,
    },
    SetBadge {
        targets: Vec<String>,
        badge: String,
        window: Option<String>,
    },
    SetColor {
        targets: Vec<String>,
        color: String,
        window: Option<String>,
    },
    SendText {
        targets: Vec<String>,
        command: String,
        window: Option<String>,
    },
    ReadText {
        tab_id: String,
        tail: Option<usize>,
        raw: Option<bool>,
    },
    WaitFor {
        tab_id: String,
        pattern: Option<String>,
        is_prompt: Option<bool>,
        from_history: Option<bool>,
        timeout_sec: Option<u64>,
    },
    ListWindows,
    CreateWindow,
    CloseWindow {
        window: String,
    },
    GetLayout {
        window: Option<String>,
    },
    ApplyYaml {
        yaml: String,
        suffix: Option<String>,
        suffix_auto: Option<bool>,
        dry_run: Option<bool>,
        window: Option<String>,
    },
    ExportLayout {
        window: Option<String>,
    },
    GetConfig,
    UpdateConfig {
        config: Value,
    },
    BuildId,
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CliResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl CliResponse {
    pub fn ok(data: Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

pub async fn handle_request(req: CliRequest, state: &AppState) -> CliResponse {
    match req {
        CliRequest::HealthCheck => CliResponse::ok(json!({ "status": "ok" })),
        CliRequest::ListTabs { window } => {
            let mut params = HashMap::new();
            if let Some(w) = window {
                params.insert("window".to_string(), w);
            }
            let tabs = crate::daemon::list_tabs(
                axum::extract::Query(params),
                axum::extract::State(state.clone()),
            )
            .await;
            CliResponse::ok(json!(tabs.0))
        }
        CliRequest::SpawnTab {
            profile,
            window,
            admin,
            elevated,
            cwd,
            cols,
            rows,
        } => {
            let payload = CreateTabRequest {
                profile,
                window,
                cols,
                rows,
                cwd,
                admin,
                elevated,
            };
            match crate::daemon::create_tab(
                axum::extract::State(state.clone()),
                Some(axum::Json(payload)),
            )
            .await
            {
                Ok(tab) => CliResponse::ok(json!(tab.0)),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::CloseTabs {
            targets,
            window,
            force,
        } => {
            let req = TargetTabRequest {
                targets,
                window,
                force,
            };
            match crate::daemon::close_tabs(
                axum::extract::State(state.clone()),
                axum::Json(req),
            )
            .await
            {
                Ok(_) => CliResponse::ok(json!({ "status": "ok" })),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::SplitTab {
            tab_id,
            direction,
            profile,
            move_tab_id,
        } => {
            let req = crate::daemon::SplitTabRequest {
                direction: Some(direction),
                profile,
                move_tab_id,
            };
            match crate::daemon::split_tab(
                axum::extract::State(state.clone()),
                axum::extract::Path(tab_id),
                Some(axum::Json(req)),
            )
            .await
            {
                Ok(res) => CliResponse::ok(json!(res.0)),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::UnsplitTab { tab_id } => {
            match crate::daemon::unsplit_tab(
                axum::extract::State(state.clone()),
                axum::extract::Path(tab_id),
            )
            .await
            {
                Ok(_) => CliResponse::ok(json!({ "status": "ok" })),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::ExplodeTab { tab_id } => {
            match crate::daemon::explode_tab(
                axum::extract::State(state.clone()),
                axum::extract::Path(tab_id),
            )
            .await
            {
                Ok(res) => CliResponse::ok(res.0),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::SetTitle {
            targets,
            title,
            window,
        } => {
            let req = SetTitleRequest { targets, title, window };
            match crate::daemon::set_title(
                axum::extract::State(state.clone()),
                axum::Json(req),
            )
            .await
            {
                Ok(_) => CliResponse::ok(json!({ "status": "ok" })),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::SetBadge {
            targets,
            badge,
            window,
        } => {
            let req = SetBadgeRequest {
                targets,
                badge,
                window,
            };
            match crate::daemon::set_badge(
                axum::extract::State(state.clone()),
                axum::Json(req),
            )
            .await
            {
                Ok(_) => CliResponse::ok(json!({ "status": "ok" })),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::SetColor {
            targets,
            color,
            window,
        } => {
            let req = SetColorRequest {
                targets,
                color,
                window,
            };
            match crate::daemon::set_color(
                axum::extract::State(state.clone()),
                axum::Json(req),
            )
            .await
            {
                Ok(_) => CliResponse::ok(json!({ "status": "ok" })),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::SendText {
            targets,
            command,
            window,
        } => {
            let req = SendTextRequest {
                targets,
                command,
                window,
            };
            match crate::daemon::send_text(
                axum::extract::State(state.clone()),
                axum::Json(req),
            )
            .await
            {
                Ok(_) => CliResponse::ok(json!({ "status": "ok" })),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::ReadText { tab_id, tail, raw } => {
            let query = ReadTabQuery { tail, raw };
            let session = match state.pty_manager.get(&tab_id) {
                Some(s) => s,
                None => return CliResponse::err("Tab not found"),
            };
            let tail_val = query.tail.unwrap_or(50);
            let raw_val = query.raw.unwrap_or(false);
            let lines = session.ring_buffer.read_tail_lines(tail_val, !raw_val);
            CliResponse::ok(json!({
                "tab_id": tab_id,
                "tail": tail_val,
                "lines": lines
            }))
        }
        CliRequest::WaitFor {
            tab_id,
            pattern,
            is_prompt,
            from_history,
            timeout_sec,
        } => {
            let req = WaitTabReq {
                pattern,
                is_prompt,
                from_history,
                timeout_sec,
            };
            let session = match state.pty_manager.get(&tab_id) {
                Some(s) => s,
                None => return CliResponse::err("Tab not found"),
            };

            let timeout_val = req.timeout_sec.unwrap_or(30);
            let prompt_val = req.is_prompt.unwrap_or(false);
            let pat_val = req.pattern.unwrap_or_default();
            let hist_val = req.from_history.unwrap_or(false);

            let start_offset = if hist_val {
                0
            } else {
                session.ring_buffer.get_total_bytes_written()
            };

            let check_matched = |s: &crate::pty::PtySession| -> bool {
                if prompt_val {
                    s.ring_buffer.matches_prompt()
                } else if !pat_val.is_empty() {
                    s.ring_buffer.contains_pattern_from_offset(start_offset, &pat_val)
                } else {
                    true
                }
            };

            if check_matched(&session) {
                return CliResponse::ok(json!({ "status": "ok", "matched": true, "elapsed_sec": 0.0 }));
            }

            let mut rx = session.tx.subscribe();
            let start_time = std::time::Instant::now();
            let timeout_duration = std::time::Duration::from_secs(timeout_val);

            let wait_res = tokio::time::timeout(timeout_duration, async {
                loop {
                    match rx.recv().await {
                        Ok(_) => {
                            if check_matched(&session) {
                                return Ok(true);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(false),
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            if check_matched(&session) {
                                return Ok(true);
                            }
                        }
                    }
                }
            })
            .await;

            match wait_res {
                Ok(Ok(true)) => CliResponse::ok(json!({
                    "status": "ok",
                    "matched": true,
                    "elapsed_sec": start_time.elapsed().as_secs_f32()
                })),
                Ok(Ok(false)) => CliResponse::err("PTY output stream closed"),
                Ok(Err(())) => unreachable!(),
                Err(_) => CliResponse::err(format!("Wait timeout elapsed ({} sec)", timeout_val)),
            }
        }
        CliRequest::ListWindows => {
            let res = crate::daemon::list_windows(axum::extract::State(state.clone())).await;
            CliResponse::ok(json!(res.0))
        }
        CliRequest::CreateWindow => {
            match crate::daemon::create_window(
                axum::extract::State(state.clone()),
            )
            .await
            {
                Ok(win) => CliResponse::ok(json!(win.0)),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::CloseWindow { window } => {
            let req = crate::daemon::CloseWindowRequest { window };
            match crate::daemon::close_window(
                axum::extract::State(state.clone()),
                axum::Json(req),
            )
            .await
            {
                Ok(_) => CliResponse::ok(json!({ "status": "ok" })),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::GetLayout { window } => {
            let mut params = HashMap::new();
            if let Some(w) = window {
                params.insert("window".to_string(), w);
            }
            let res = crate::daemon::get_window_layout(
                axum::extract::State(state.clone()),
                axum::extract::Query(params),
            )
            .await;
            CliResponse::ok(json!(res.0))
        }
        CliRequest::ApplyYaml {
            yaml,
            suffix,
            suffix_auto,
            dry_run,
            window,
        } => {
            let req = ApplySessionRequest {
                yaml: Some(yaml),
                file: None,
                suffix,
                suffix_auto,
                dry_run,
                window,
            };
            match crate::daemon::apply_session(
                axum::extract::State(state.clone()),
                axum::Json(req),
            )
            .await
            {
                Ok(res) => CliResponse::ok(res.0),
                Err((_, err)) => CliResponse::err(err),
            }
        }
        CliRequest::ExportLayout { window } => {
            let win_id = window.unwrap_or_else(|| "win-1".to_string());
            let yaml = crate::exporter::export_yaml_layout(state, &win_id);
            CliResponse::ok(json!(yaml))
        }
        CliRequest::GetConfig => {
            let cfg = crate::config::AppConfig::load();
            CliResponse::ok(json!(cfg))
        }
        CliRequest::UpdateConfig { config } => {
            match serde_json::from_value::<crate::config::AppConfig>(config) {
                Ok(cfg) => match cfg.save() {
                    Ok(_) => CliResponse::ok(json!({ "status": "ok" })),
                    Err(e) => CliResponse::err(e.to_string()),
                },
                Err(e) => CliResponse::err(format!("Invalid config structure: {}", e)),
            }
        }
        CliRequest::BuildId => CliResponse::ok(json!({ "build_id": env!("CARGO_PKG_VERSION") })),
        CliRequest::Shutdown => {
            tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                std::process::exit(0);
            });
            CliResponse::ok(json!({ "status": "shutting down" }))
        }
    }
}

pub async fn start_pipe_server(state: AppState) {
    #[cfg(windows)]
    {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::windows::named_pipe::ServerOptions;

        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(PIPE_NAME);

        loop {
            match server {
                Ok(pipe) => {
                    let state_clone = state.clone();
                    let connection_result = pipe.connect().await;

                    let next_server = ServerOptions::new().create(PIPE_NAME);

                    if connection_result.is_ok() {
                        tokio::spawn(async move {
                            let (reader, mut writer) = tokio::io::split(pipe);
                            let mut buf_reader = BufReader::new(reader);
                            let mut line = String::new();

                            while let Ok(n) = buf_reader.read_line(&mut line).await {
                                if n == 0 {
                                    break;
                                }
                                let response = match serde_json::from_str::<CliRequest>(&line) {
                                    Ok(req) => handle_request(req, &state_clone).await,
                                    Err(e) => CliResponse::err(format!("Invalid JSON-RPC payload: {}", e)),
                                };
                                let mut resp_str = serde_json::to_string(&response).unwrap();
                                resp_str.push('\n');
                                if writer.write_all(resp_str.as_bytes()).await.is_err() {
                                    break;
                                }
                                let _ = writer.flush().await;
                                line.clear();
                            }
                        });
                    }
                    server = next_server;
                }
                Err(e) => {
                    tracing::error!("[named_pipe] Error creating pipe instance: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    server = ServerOptions::new().create(PIPE_NAME);
                }
            }
        }
    }
}
