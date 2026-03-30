use std::{
    collections::BTreeSet,
    env,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use tracing::debug;
use zellij_utils::consts::CLIENT_SERVER_CONTRACT_DIR;

use super::{ActionReplyMode, ZellijSessionManager};
use crate::proto_ipc;

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
        let mut sessions = BTreeSet::new();

        for candidate in Self::candidate_socket_paths(socket_path) {
            if let Some(name) = candidate.file_name().and_then(|n| n.to_str()) {
                let manager = Self::new(name.to_string(), candidate.clone())?;
                if manager.is_alive() {
                    sessions.insert(name.to_string());
                }
            }
        }

        Ok(sessions.into_iter().collect())
    }

    pub fn find_session_socket(socket_dir: &Path, session_name: &str) -> Option<PathBuf> {
        Self::candidate_socket_paths(socket_dir)
            .into_iter()
            .find(|path| path.file_name().and_then(|n| n.to_str()) == Some(session_name))
    }

    pub fn create_background_session(
        zellij_path: &Path,
        socket_dir: &Path,
        session_name: &str,
    ) -> Result<bool> {
        if let Some(socket_path) = Self::find_session_socket(socket_dir, session_name) {
            if Self::session_socket_is_alive(session_name, &socket_path) {
                return Ok(false);
            }
        }

        let socket_dir = Self::socket_dir_for_creation(socket_dir);
        std::fs::create_dir_all(&socket_dir)?;
        let socket_path = socket_dir.join(session_name);

        let mut child = Command::new(zellij_path)
            .arg("--server")
            .arg(&socket_path)
            .env("ZELLIJ", "0")
            .env("ZELLIJ_SESSION_NAME", session_name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut initialized = false;
        loop {
            if !initialized {
                match proto_ipc::ProtoIpcConnection::connect(&socket_path) {
                    Ok(mut conn) => {
                        conn.send_client_msg(&proto_ipc::first_client_connected_request(
                            env::current_dir().ok(),
                        ))?;
                        initialized = true;
                    }
                    Err(_) => {}
                }
            }

            if let Some(socket_path) = Self::find_session_socket(&socket_dir, session_name) {
                if Self::session_socket_is_alive(session_name, &socket_path) {
                    let _ = child.try_wait();
                    return Ok(true);
                }
            }

            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    anyhow::bail!(
                        "Failed to create session '{}': zellij server exited with status {}",
                        session_name,
                        status
                    );
                }
            }

            if Instant::now() >= deadline {
                anyhow::bail!(
                    "Timed out waiting for session '{}' to become available",
                    session_name
                );
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    fn candidate_search_roots(socket_dir: &Path) -> Vec<PathBuf> {
        let mut roots = vec![socket_dir.to_path_buf()];

        if socket_dir.file_name().and_then(|n| n.to_str()) != Some("zellij") {
            if let Some(parent) = socket_dir.parent() {
                roots.push(parent.to_path_buf());
            }
        }

        roots
    }

    fn candidate_socket_paths(socket_dir: &Path) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        let mut seen = BTreeSet::new();

        for root in Self::candidate_search_roots(socket_dir) {
            if !root.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&root) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    if Self::is_socket_path(&path) && seen.insert(path.clone()) {
                        candidates.push(path.clone());
                    }

                    if path.is_dir() {
                        if let Ok(children) = std::fs::read_dir(&path) {
                            for child in children.flatten() {
                                let child_path = child.path();
                                if Self::is_socket_path(&child_path)
                                    && seen.insert(child_path.clone())
                                {
                                    candidates.push(child_path);
                                }
                            }
                        }
                    }
                }
            }
        }

        candidates
    }

    fn socket_dir_for_creation(socket_dir: &Path) -> PathBuf {
        let contract_dir = CLIENT_SERVER_CONTRACT_DIR.as_str();

        if socket_dir.file_name().and_then(|n| n.to_str()) == Some(contract_dir) {
            return socket_dir.to_path_buf();
        }

        if socket_dir.file_name().and_then(|n| n.to_str()) == Some("zellij") {
            return socket_dir.join(contract_dir);
        }

        if let Some(parent) = socket_dir.parent() {
            if parent.file_name().and_then(|n| n.to_str()) == Some("zellij") {
                return parent.join(contract_dir);
            }
        }

        socket_dir.to_path_buf()
    }

    fn is_socket_path(path: &Path) -> bool {
        std::fs::metadata(path)
            .map(|metadata| metadata.file_type().is_socket())
            .unwrap_or(false)
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
            vec![proto_ipc::rename_session_action(new_name.clone())],
            ActionReplyMode::UnblockOrLog,
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
