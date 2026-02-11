use std::{os::unix::fs::FileTypeExt, path::PathBuf};

use anyhow::{Context, Result};
use interprocess::local_socket::LocalSocketStream;
use tracing::{debug, warn};
use zellij_utils::ipc::{
    ClientToServerMsg, IpcReceiverWithContext, IpcSenderWithContext, ServerToClientMsg,
};

#[derive(Debug, Clone)]
pub struct ZellijSessionManager {
    /// Session Name
    session_name: String,
    /// Socket Path
    socket_path: PathBuf,
}

impl ZellijSessionManager {
    /// Create a new session manager
    pub fn new(session_name: String, socket_path: PathBuf) -> Result<Self> {
        debug!(
            "Created session manager for '{}' at {:?}",
            session_name, socket_path
        );

        Ok(Self {
            session_name,
            socket_path,
        })
    }

    /// Current Session Name
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    /// Socket Path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
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

    fn connect(
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

    pub fn list_sessions(socket_path: &PathBuf) -> Result<Vec<String>> {
        let mut sessions = Vec::new();

        for entry in std::fs::read_dir(&socket_path)? {
            let entry = entry?;
            let path = entry.path();
            if let Ok(metadata) = std::fs::metadata(&path) {
                if !metadata.file_type().is_socket() {
                    continue;
                }

                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let manager = Self::new(name.to_string(), path.clone())?;
                    if manager.is_alive() {
                        sessions.push(name.to_string());
                    }
                }
            }
        }

        Ok(sessions)
    }
}
