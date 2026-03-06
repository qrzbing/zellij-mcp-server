use std::path::PathBuf;

use crate::manager::ZellijSessionManager;

#[derive(Debug)]
pub struct CliContext<'a> {
    /// Zellij Socket Address
    pub socket_dir: &'a PathBuf,

    /// Zellij Session Manager
    pub manager: Option<ZellijSessionManager>,

    /// Zellij Executable Path
    pub zellij_path: &'a PathBuf,
}

impl<'a> CliContext<'a> {
    pub fn new(socket_dir: &'a PathBuf, zellij_path: &'a PathBuf) -> Self {
        Self {
            socket_dir,
            manager: None,
            zellij_path,
        }
    }

    pub fn attach(&mut self, session_name: String) -> anyhow::Result<()> {
        let manager =
            ZellijSessionManager::new(session_name.clone(), self.socket_dir.join(&session_name))?;
        if !manager.is_alive() {
            anyhow::bail!("Session '{}' is not running", session_name);
        }
        self.manager = Some(manager);

        Ok(())
    }

    pub fn detach(&mut self) {
        self.manager = None;
    }

    pub fn current_session_name(&self) -> Option<&str> {
        match self.manager {
            Some(ref manager) => Some(manager.session_name()),
            None => None,
        }
    }

    pub fn is_attached(&self) -> bool {
        self.manager.is_some()
    }

    pub fn manager(&self) -> Option<&ZellijSessionManager> {
        self.manager.as_ref()
    }

    pub fn manager_mut(&mut self) -> Option<&mut ZellijSessionManager> {
        self.manager.as_mut()
    }
}
