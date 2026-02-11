use anyhow::{Context, Result};
use interprocess::local_socket::LocalSocketStream;
use tracing::{debug, warn};
use zellij_utils::{
    cli::CliAction,
    input::actions::Action,
    ipc::{ClientToServerMsg, IpcReceiverWithContext, IpcSenderWithContext, ServerToClientMsg},
};

use super::ZellijSessionManager;

impl ZellijSessionManager {
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

        let stream = LocalSocketStream::connect(self.socket_path.clone()).context(format!(
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
        match self.connect() {
            Ok((mut sender, mut receiver)) => {
                if let Err(e) = sender.send(ClientToServerMsg::ConnStatus) {
                    warn!("Failed to send ConnStatus: {}", e);
                    return false;
                }

                match receiver.recv() {
                    Some((ServerToClientMsg::Connected, _)) => {
                        debug!("Session '{}' is alive", self.session_name);
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
            Err(e) => {
                debug!(
                    "Failed to connect to session '{}': {}",
                    self.session_name, e
                );
                false
            }
        }
    }

    pub fn send_action(&self, cli_action: CliAction, terminal_id: Option<u32>) -> Result<String> {
        let (mut sender, mut receiver) = self.connect()?;
        let actions = Action::actions_from_cli(
            cli_action,
            Box::new(|| std::env::current_dir().unwrap_or_default()),
            None,
        )
        .map_err(|e| anyhow::anyhow!("Failed to convert action: {}", e))?;

        for action in actions {
            sender
                .send(ClientToServerMsg::Action(action, terminal_id, None))
                .context("Failed to send action message")?;
        }

        let mut output = String::new();
        loop {
            match receiver.recv() {
                Some((ServerToClientMsg::UnblockInputThread, _)) => {
                    debug!("Action completed successfully");
                    break;
                }

                Some((ServerToClientMsg::Log(lines), _)) => {
                    let log_output = lines.join("\n");
                    debug!("Received log output: {}", log_output);
                    output.push_str(&log_output);
                    break;
                }

                Some((ServerToClientMsg::LogError(lines), _)) => {
                    let error_output = lines.join("\n");
                    anyhow::bail!("Action error: {}", error_output);
                }

                Some((ServerToClientMsg::Exit(exit_reason), _)) => {
                    use zellij_utils::ipc::ExitReason;
                    match exit_reason {
                        ExitReason::Error(e) => {
                            anyhow::bail!("Exit with error: {}", e);
                        }
                        _ => {
                            debug!("Action completed with exit");
                            break;
                        }
                    }
                }

                None => {
                    warn!("Connection closed unexpectedly");
                    anyhow::bail!("Connection closed before action completed");
                }

                Some((msg, _)) => {
                    debug!("Ignoring message: {:?}", msg);
                }
            }
        }

        let _ = sender.send(ClientToServerMsg::ClientExited);

        Ok(output)
    }
}
