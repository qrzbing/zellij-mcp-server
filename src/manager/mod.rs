use std::path::PathBuf;

use anyhow::Context;
use tracing::debug;

mod middleware;
mod session;
mod tab;

#[derive(Debug, Clone)]
pub struct ZellijSessionManager {
    /// Session Name
    session_name: String,
    /// Socket Path
    socket_path: PathBuf,
    /// Current Tab Name
    current_tab_name: Option<String>,
}

impl ZellijSessionManager {
    /// Create a new session manager
    pub fn new(session_name: String, socket_path: PathBuf) -> anyhow::Result<Self> {
        debug!(
            "Created session manager for '{}' at {:?}",
            session_name, socket_path
        );

        let mut mgr = Self {
            session_name: session_name.clone(),
            socket_path,
            current_tab_name: None,
        };

        mgr.refresh_current_tab().with_context(|| {
            format!(
                "Failed to initialize tab information for session '{}'",
                session_name
            )
        })?;

        debug!("Current panel: {:?}", mgr.current_tab_name());

        Ok(mgr)
    }
}
