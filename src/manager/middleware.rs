use anyhow::{Context, Result};
use tracing::{debug, warn};
use zellij_utils::{
    cli::CliAction,
    consts::ipc_connect,
    input::{actions::Action, cli_assets::CliAssets},
    ipc::{ClientToServerMsg, IpcReceiverWithContext, IpcSenderWithContext, ServerToClientMsg},
    pane_size::Size,
};

use super::ZellijSessionManager;

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
        receiver: &mut IpcReceiverWithContext<ServerToClientMsg>,
        mode: ActionReplyMode,
        label: &str,
    ) -> Result<String> {
        let mut output = String::new();
        loop {
            match receiver.recv_server_msg() {
                Some((ServerToClientMsg::UnblockInputThread, _)) => {
                    if mode == ActionReplyMode::UnblockOrLog {
                        debug!("{label} completed on UnblockInputThread");
                        break;
                    } else {
                        debug!("{label} ignoring UnblockInputThread in LogOnly mode");
                    }
                }

                Some((ServerToClientMsg::Log { lines }, _)) => {
                    let log_output = lines.join("\n");
                    debug!("{label} received log output: {}", log_output);
                    output.push_str(&log_output);
                    break;
                }

                Some((ServerToClientMsg::LogError { lines }, _)) => {
                    let error_output = lines.join("\n");
                    anyhow::bail!("{label} error: {}", error_output);
                }

                Some((ServerToClientMsg::Exit { exit_reason }, _)) => {
                    use zellij_utils::ipc::ExitReason;
                    match exit_reason {
                        ExitReason::Error(e) => {
                            anyhow::bail!("{label} exited with error: {}", e);
                        }
                        _ => {
                            debug!("{label} completed with exit");
                            break;
                        }
                    }
                }

                None => {
                    warn!("{label} connection closed unexpectedly");
                    anyhow::bail!("Connection closed before action completed");
                }

                Some((msg, _)) => {
                    debug!("{label} ignoring message: {:?}", msg);
                }
            }
        }
        Ok(output)
    }

    fn wait_attach_barrier(receiver: &mut IpcReceiverWithContext<ServerToClientMsg>) -> Result<()> {
        loop {
            match receiver.recv_server_msg() {
                Some((ServerToClientMsg::UnblockInputThread, _)) => {
                    debug!("Attach barrier completed on UnblockInputThread");
                    break;
                }
                Some((ServerToClientMsg::Exit { exit_reason }, _)) => {
                    use zellij_utils::ipc::ExitReason;
                    match exit_reason {
                        ExitReason::Error(e) => {
                            anyhow::bail!("Attach failed with error: {}", e);
                        }
                        _ => {
                            debug!("Attach barrier completed with exit");
                            break;
                        }
                    }
                }
                None => {
                    warn!("Attach barrier connection closed unexpectedly");
                    anyhow::bail!("Connection closed before attach completed");
                }
                Some((msg, _)) => {
                    // Attach 期间可能收到 Render/Log 等消息，这里仅作为 barrier 消耗掉
                    debug!("Attach barrier ignoring message: {:?}", msg);
                }
            }
        }
        Ok(())
    }

    pub(super) fn connect(
        &self,
    ) -> Result<(
        IpcSenderWithContext<ClientToServerMsg>,
        IpcReceiverWithContext<ServerToClientMsg>,
    )> {
        if !self.socket_path.exists() {
            anyhow::bail!(
                "Session '{}' socket does not exist: {:?}",
                self.session_name,
                self.socket_path
            );
        }

        let stream = ipc_connect(&self.socket_path).context(format!(
            "Failed to connect to session '{}' at {:?}",
            self.session_name, self.socket_path
        ))?;

        debug!(
            "Connected to session '{}' at {:?}",
            self.session_name, self.socket_path
        );

        let sender = IpcSenderWithContext::<ClientToServerMsg>::new(stream);
        let receiver = sender.get_receiver();

        Ok((sender, receiver))
    }

    pub fn is_alive(&self) -> bool {
        let socket_path = self.socket_path.clone();
        let session_name = self.session_name.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let alive = match ipc_connect(&socket_path) {
                Ok(stream) => {
                    let mut sender = IpcSenderWithContext::<ClientToServerMsg>::new(stream);
                    if let Err(e) = sender.send_client_msg(ClientToServerMsg::ConnStatus) {
                        warn!("Failed to send ConnStatus: {}", e);
                        false
                    } else {
                        let mut receiver: IpcReceiverWithContext<ServerToClientMsg> =
                            sender.get_receiver();
                        match receiver.recv_server_msg() {
                            Some((ServerToClientMsg::Connected, _)) => {
                                debug!("Session '{}' is alive", session_name);
                                true
                            }
                            Some((msg, _)) => {
                                warn!("Unexpected response to ConnStatus: {:?}", msg);
                                false
                            }
                            None => {
                                warn!("No response to ConnStatus");
                                false
                            }
                        }
                    }
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
        let (mut sender, mut receiver) = self.connect()?;
        let reply_mode = Self::reply_mode_for(&cli_action);

        let result = (|| {
            let actions = Action::actions_from_cli(
                cli_action,
                Box::new(|| std::env::current_dir().unwrap_or_default()),
                None,
            )
            .map_err(|e| anyhow::anyhow!("Failed to convert action: {}", e))?;

            for action in actions {
                sender
                    .send_client_msg(ClientToServerMsg::Action {
                        action,
                        terminal_id,
                        client_id: None,
                        is_cli_client: true,
                    })
                    .context("Failed to send action message")?;
            }

            Self::wait_action_result(&mut receiver, reply_mode, "Action")
        })();

        let _ = sender.send_client_msg(ClientToServerMsg::ClientExited);

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
        let (mut sender, mut receiver) = self.connect()?;
        let reply_mode = Self::reply_mode_for(&cli_action);
        let cli_assets = CliAssets {
            terminal_window_size: Size {
                rows: 4096,
                cols: 4096,
            },
            cwd: std::env::current_dir().ok(),
            ..Default::default()
        };

        let result = (|| {
            // Step 1: 注册为 UI 客户端
            // 使用超大尺寸，确保 min_client_terminal_size() 不会影响其他已连接的真实终端
            sender
                .send_client_msg(ClientToServerMsg::AttachClient {
                    cli_assets: cli_assets.clone(),
                    tab_position_to_focus,
                    pane_to_focus: None,
                    is_web_client: false,
                })
                .context("Failed to send AttachClient")?;

            // Step 2: attach barrier - 显式等待 attach 阶段完成，避免依赖 sleep
            Self::wait_attach_barrier(&mut receiver)?;

            // Step 3: 发送实际 action
            let actions = Action::actions_from_cli(
                cli_action,
                Box::new(|| std::env::current_dir().unwrap_or_default()),
                None,
            )
            .map_err(|e| anyhow::anyhow!("Failed to convert action: {}", e))?;

            for action in actions {
                sender
                    .send_client_msg(ClientToServerMsg::Action {
                        action,
                        terminal_id,
                        client_id: None,
                        is_cli_client: false,
                    })
                    .context("Failed to send action message")?;
            }

            // Step 4: 按 action 语义等待响应
            Self::wait_action_result(&mut receiver, reply_mode, "UI action")
        })();

        // Step 5: 正确退出，服务端移除我们的状态并触发 resize 恢复
        let _ = sender.send_client_msg(ClientToServerMsg::ClientExited);

        result
    }
}
