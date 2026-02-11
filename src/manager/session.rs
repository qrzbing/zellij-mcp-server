use std::{os::unix::fs::FileTypeExt, path::PathBuf};

use anyhow::Result;

use super::ZellijSessionManager;

impl ZellijSessionManager {
    /// Current Session Name
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    /// Socket Path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
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
