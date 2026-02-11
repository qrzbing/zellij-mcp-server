use std::path::PathBuf;

use crate::manager::session::ZellijSessionManager;

#[derive(Debug)]
pub struct CliContext {
    /// Zellij Socket Address
    pub socket_dir: PathBuf,

    /// Zellij Session Manager
    pub manager: Option<ZellijSessionManager>,

    /// Zellij Executable Path
    pub zellij_path: String,
}

impl CliContext {
    pub fn new(socket_dir: PathBuf, zellij_path: String) -> Self {
        Self {
            socket_dir,
            manager: None,
            zellij_path,
        }
    }

    pub fn current_session_name(&self) -> Option<&str> {
        match self.manager {
            Some(ref manager) => Some(manager.session_name()),
            None => None,
        }
    }
}
