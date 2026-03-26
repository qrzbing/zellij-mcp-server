use std::{
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use tracing::debug;
use zellij_utils::cli::CliAction;

use super::ZellijSessionManager;

impl ZellijSessionManager {
    /// Current Session Name
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    // /// Socket Path
    // pub fn socket_path(&self) -> &PathBuf {
    //     &self.socket_path
    // }

    pub fn list_sessions(socket_path: &PathBuf) -> Result<Vec<String>> {
        if !socket_path.exists() {
            return Ok(Vec::new());
        }

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

    pub fn create_background_session(
        zellij_path: &Path,
        socket_dir: &Path,
        session_name: &str,
    ) -> Result<bool> {
        let socket_path = socket_dir.join(session_name);
        if Self::session_socket_is_alive(session_name, &socket_path) {
            return Ok(false);
        }

        let mut child = Command::new(zellij_path)
            .arg("attach")
            .arg("--create-background")
            .arg(session_name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if Self::session_socket_is_alive(session_name, &socket_path) {
                if child.try_wait()?.is_none() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                return Ok(true);
            }

            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    anyhow::bail!(
                        "Failed to create session '{}': zellij exited with status {}",
                        session_name,
                        status
                    );
                }
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!(
                    "Timed out waiting for session '{}' to become available",
                    session_name
                );
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    fn session_socket_is_alive(session_name: &str, socket_path: &Path) -> bool {
        if !socket_path.exists() {
            return false;
        }

        Self::new(session_name.to_string(), socket_path.to_path_buf())
            .map(|manager| manager.is_alive())
            .unwrap_or(false)
    }

    /// Rename the current session
    pub fn rename_session(&mut self, new_name: String) -> Result<()> {
        self.send_action(
            CliAction::RenameSession {
                name: new_name.clone(),
            },
            None,
        )?;

        let socket_dir = self
            .socket_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow::anyhow!("Session socket has no parent directory"))?;
        self.session_name = new_name.clone();
        self.socket_path = socket_dir.join(&new_name);

        debug!("Renamed session to: {}", new_name);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ZellijSessionManager;

    #[test]
    fn list_sessions_returns_empty_when_socket_dir_is_missing() {
        let path = PathBuf::from("/tmp/zellij-mcp-server-test-missing-socket-dir");
        if path.exists() {
            std::fs::remove_dir_all(&path).unwrap();
        }

        let sessions = ZellijSessionManager::list_sessions(&path).unwrap();

        assert!(sessions.is_empty());
    }
}
