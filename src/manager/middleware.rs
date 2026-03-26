use anyhow::{Context, Result};
use tracing::{debug, warn};
use zellij_utils::{
    cli::CliAction,
};

use super::ZellijSessionManager;
use crate::proto_ipc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionReplyMode {
    /// `Log` / `LogError` / `Exit` 才结束，忽略 `UnblockInputThread`
    LogOnly,
    /// `UnblockInputThread` 或 `Log` 都可以结束
    UnblockOrLog,
}

impl ZellijSessionManager {
    fn reply_mode_for(cli_action: &CliAction) -> ActionReplyMode {
        match cli_action {
            CliAction::QueryTabNames | CliAction::DumpLayout | CliAction::ListClients => {
                ActionReplyMode::LogOnly
            }
            _ => ActionReplyMode::UnblockOrLog,
        }
    }

    fn wait_action_result(
        conn: &mut proto_ipc::ProtoIpcConnection,
        mode: ActionReplyMode,
        label: &str,
    ) -> Result<String> {
        let mut output = String::new();
        loop {
            match conn.recv_server_msg()?.message {
                Some(proto_ipc::ProtoServerMessage::UnblockInputThread(_)) => {
                    if mode == ActionReplyMode::UnblockOrLog {
                        debug!("{label} completed on UnblockInputThread");
                        break;
                    } else {
                        debug!("{label} ignoring UnblockInputThread in LogOnly mode");
                    }
                }

                Some(proto_ipc::ProtoServerMessage::Log(log)) => {
                    let log_output = log.lines.join("\n");
                    debug!("{label} received log output: {}", log_output);
                    output.push_str(&log_output);
                    break;
                }

                Some(proto_ipc::ProtoServerMessage::LogError(log_error)) => {
                    let error_output = log_error.lines.join("\n");
                    anyhow::bail!("{label} error: {}", error_output);
                }

                Some(proto_ipc::ProtoServerMessage::Exit(exit)) => {
                    match proto_ipc::ProtoExitReason::from_i32(exit.exit_reason) {
                        Some(proto_ipc::ProtoExitReason::Error) => {
                            let error = exit.payload.unwrap_or_else(|| "unknown error".to_string());
                            anyhow::bail!("{label} exited with error: {}", error);
                        }
                        Some(_) | None => {
                            debug!("{label} completed with exit");
                            break;
                        }
                    }
                }

                None => {
                    warn!("{label} connection closed unexpectedly");
                    anyhow::bail!("Connection closed before action completed");
                }

                Some(_) => {
                    debug!("{label} ignoring non-terminal response");
                }
            }
        }
        Ok(output)
    }

    fn wait_attach_barrier(conn: &mut proto_ipc::ProtoIpcConnection) -> Result<()> {
        loop {
            match conn.recv_server_msg()?.message {
                Some(proto_ipc::ProtoServerMessage::UnblockInputThread(_)) => {
                    debug!("Attach barrier completed on UnblockInputThread");
                    break;
                }
                Some(proto_ipc::ProtoServerMessage::Exit(exit)) => {
                    match proto_ipc::ProtoExitReason::from_i32(exit.exit_reason) {
                        Some(proto_ipc::ProtoExitReason::Error) => {
                            let error = exit.payload.unwrap_or_else(|| "unknown error".to_string());
                            anyhow::bail!("Attach failed with error: {}", error);
                        }
                        Some(_) | None => {
                            debug!("Attach barrier completed with exit");
                            break;
                        }
                    }
                }
                None => {
                    warn!("Attach barrier connection closed unexpectedly");
                    anyhow::bail!("Connection closed before attach completed");
                }
                Some(_) => {
                    // Attach 期间可能收到 Render/Log 等消息，这里仅作为 barrier 消耗掉
                    debug!("Attach barrier ignoring non-terminal response");
                }
            }
        }
        Ok(())
    }

    fn connect_proto(&self) -> Result<proto_ipc::ProtoIpcConnection> {
        if !self.socket_path.exists() {
            anyhow::bail!(
                "Session '{}' socket does not exist: {:?}",
                self.session_name,
                self.socket_path
            );
        }

        let conn = proto_ipc::ProtoIpcConnection::connect(&self.socket_path).context(format!(
            "Failed to connect to session '{}' at {:?}",
            self.session_name, self.socket_path
        ))?;

        debug!(
            "Connected to session '{}' at {:?}",
            self.session_name, self.socket_path
        );

        Ok(conn)
    }

    pub fn is_alive(&self) -> bool {
        let socket_path = self.socket_path.clone();
        let session_name = self.session_name.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let alive = match proto_ipc::probe_session(&socket_path) {
                Ok(alive) => {
                    if alive {
                        debug!("Session '{}' is alive", session_name);
                    } else {
                        warn!("Unexpected response to ConnStatus for session '{}'", session_name);
                    }
                    alive
                }
                Err(e) => {
                    debug!("Failed to connect to session '{}': {}", session_name, e);
                    false
                }
            };
            let _ = tx.send(alive);
        });

        match rx.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(alive) => alive,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                warn!(
                    "Timed out waiting for ConnStatus response from session '{}'",
                    self.session_name
                );
                false
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => false,
        }
    }

    pub fn send_action(&self, cli_action: CliAction, terminal_id: Option<u32>) -> Result<String> {
        let mut conn = self.connect_proto()?;
        let reply_mode = Self::reply_mode_for(&cli_action);

        let result = (|| {
            let actions = proto_ipc::actions_from_cli(cli_action)?;

            for action in actions {
                conn.send_client_msg(&proto_ipc::action_request(
                    action,
                    terminal_id,
                    None,
                    true,
                ))
                    .context("Failed to send action message")?;
            }

            Self::wait_action_result(&mut conn, reply_mode, "Action")
        })();

        let _ = conn.send_client_msg(&proto_ipc::client_exited_request());

        result
    }

    /// 以 UI 客户端身份发送 action。
    ///
    /// 与 `send_action` 的区别：先发送 `AttachClient` 将自身注册为正式 UI 客户端，
    /// 从而让 `DumpScreen`、`GoToTabName` 等依赖 `active_tab_indices` 的操作能正常工作。
    ///
    /// 副作用：短暂触发一次 resize（使用 9999x9999 大尺寸，不会缩小真实终端）。
    pub fn send_action_as_ui_client(
        &self,
        cli_action: CliAction,
        terminal_id: Option<u32>,
        tab_position_to_focus: Option<usize>,
    ) -> Result<String> {
        let mut conn = self.connect_proto()?;
        let reply_mode = Self::reply_mode_for(&cli_action);

        let result = (|| {
            // Step 1: 注册为 UI 客户端
            // 使用超大尺寸，确保 min_client_terminal_size() 不会影响其他已连接的真实终端
            conn.send_client_msg(&proto_ipc::attach_client_request(
                std::env::current_dir().ok(),
                tab_position_to_focus,
            )?)
                .context("Failed to send AttachClient")?;

            // Step 2: attach barrier - 显式等待 attach 阶段完成，避免依赖 sleep
            Self::wait_attach_barrier(&mut conn)?;

            // Step 3: 发送实际 action
            let actions = proto_ipc::actions_from_cli(cli_action)?;

            for action in actions {
                conn.send_client_msg(&proto_ipc::action_request(
                    action,
                    terminal_id,
                    None,
                    false,
                ))
                    .context("Failed to send action message")?;
            }

            // Step 4: 按 action 语义等待响应
            Self::wait_action_result(&mut conn, reply_mode, "UI action")
        })();

        // Step 5: 正确退出，服务端移除我们的状态并触发 resize 恢复
        let _ = conn.send_client_msg(&proto_ipc::client_exited_request());

        result
    }
}
