use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;

use crate::manager::ZellijSessionManager;

/// Zellij Session Manager for MCP
///
/// Manages multiple Zellij session instances, allowing attach/detach
/// and tracking the current active session.
#[derive(Clone)]
pub struct SessionManager {
    instances: Arc<Mutex<HashMap<String, ZellijSessionManager>>>,
    current_session: Arc<Mutex<Option<String>>>,
    socket_dir: PathBuf,
    zellij_path: PathBuf,
}

impl SessionManager {
    pub fn new(socket_dir: PathBuf, zellij_path: PathBuf) -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
            current_session: Arc::new(Mutex::new(None)),
            socket_dir,
            zellij_path,
        }
    }

    /// Attach to a Zellij session
    pub fn attach(&self, session_name: String) -> Result<()> {
        let mut instances = self.instances.lock().unwrap();

        if instances.contains_key(&session_name) {
            anyhow::bail!("Already attached to session '{}'", session_name);
        }

        let socket_path = self.socket_dir.join(&session_name);
        if !socket_path.exists() {
            anyhow::bail!("Session '{}' not found", session_name);
        }

        let manager = ZellijSessionManager::new(session_name.clone(), socket_path)?;
        if !manager.is_alive() {
            anyhow::bail!("Session '{}' is not running", session_name);
        }

        instances.insert(session_name.clone(), manager);
        drop(instances);

        // Automatically set as current session
        self.set_current_session(Some(session_name))?;

        Ok(())
    }

    /// Create a detached session if missing, then attach to it.
    pub fn new_session(&self, session_name: String) -> Result<bool> {
        let created = ZellijSessionManager::create_background_session(
            &self.zellij_path,
            &self.socket_dir,
            &session_name,
        )?;

        {
            let instances = self.instances.lock().unwrap();
            if instances.contains_key(&session_name) {
                drop(instances);
                self.set_current_session(Some(session_name))?;
                return Ok(created);
            }
        }

        self.attach(session_name)?;
        Ok(created)
    }

    /// Detach from a Zellij session
    pub fn detach(&self, session_name: &str) -> Result<()> {
        let mut instances = self.instances.lock().unwrap();
        instances
            .remove(session_name)
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", session_name))?;

        // If detaching the current session, clear the state
        drop(instances);
        let mut current = self.current_session.lock().unwrap();
        if current.as_ref().map(|s| s.as_str()) == Some(session_name) {
            *current = None;
            tracing::info!("Cleared current session (detached '{}')", session_name);
        }

        Ok(())
    }

    /// List all attached sessions
    pub fn list_attached(&self) -> Vec<String> {
        let instances = self.instances.lock().unwrap();
        instances.keys().cloned().collect()
    }

    /// List all available Zellij sessions
    pub fn list_available(&self) -> Result<Vec<String>> {
        ZellijSessionManager::list_sessions(&self.socket_dir)
    }

    /// Set the current active session
    pub fn set_current_session(&self, name: Option<String>) -> Result<()> {
        if let Some(ref n) = name {
            let instances = self.instances.lock().unwrap();
            if !instances.contains_key(n) {
                anyhow::bail!("Session '{}' not attached", n);
            }
        }

        let mut current = self.current_session.lock().unwrap();
        *current = name.clone();

        if let Some(n) = name {
            tracing::info!("Set current session to '{}'", n);
        } else {
            tracing::info!("Cleared current session");
        }

        Ok(())
    }

    /// Get the current session name
    pub fn get_current_session_name(&self) -> Option<String> {
        let current = self.current_session.lock().unwrap();
        current.clone()
    }

    fn resolve_name(&self, name_opt: Option<String>) -> Result<String> {
        match name_opt {
            Some(n) => Ok(n),
            None => {
                let current = self.current_session.lock().unwrap();
                current.as_ref().cloned().ok_or_else(|| {
                    anyhow::anyhow!(
                        "No session name provided and no current session is set. \
                         Use 'attach_session' to attach to a session first."
                    )
                })
            }
        }
    }

    pub fn with_session<T, F>(&self, name_opt: Option<String>, f: F) -> Result<T>
    where
        F: FnOnce(&ZellijSessionManager) -> Result<T>,
    {
        let name = self.resolve_name(name_opt)?;
        let instances = self.instances.lock().unwrap();
        let mgr = instances
            .get(&name)
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not attached", name))?;
        f(mgr)
    }

    pub fn with_session_mut<T, F>(&self, name_opt: Option<String>, f: F) -> Result<T>
    where
        F: FnOnce(&mut ZellijSessionManager) -> Result<T>,
    {
        let name = self.resolve_name(name_opt)?;
        let mut instances = self.instances.lock().unwrap();
        let mgr = instances
            .get_mut(&name)
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not attached", name))?;
        f(mgr)
    }

    // Set log directory for a session, modifying the stored instance directly
    pub fn set_session_log_dir(&self, name_opt: Option<String>, dir: PathBuf) -> Result<()> {
        let name = match name_opt {
            Some(n) => n,
            None => {
                let current = self.current_session.lock().unwrap();
                current.as_ref().cloned().ok_or_else(|| {
                    anyhow::anyhow!(
                        "No session name provided and no current session is set. \
                       Use 'attach_session' to attach to a session first."
                    )
                })?
            }
        };

        let mut instances = self.instances.lock().unwrap();
        let mgr = instances
            .get_mut(&name)
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not attached", name))?;
        mgr.set_log_dir(dir)
    }
}
