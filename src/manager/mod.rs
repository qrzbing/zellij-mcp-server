use std::path::PathBuf;

use tracing::{debug, warn};

mod middleware;
pub mod readwrite;
mod session;
mod tab;
mod utils;

// Re-export utility functions for use in other modules
pub use utils::{format_key_name, parse_key_string};

#[derive(Debug, Clone)]
pub struct ZellijSessionManager {
    /// Session Name
    session_name: String,
    /// Socket Path
    socket_path: PathBuf,
    /// Current Tab Name
    current_tab_name: Option<String>,
    /// Dump screen to directory in this session
    dump_screen_dir: Option<PathBuf>,
    /// Last Dump Message
    last_dump_message: Option<Vec<String>>,
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
            dump_screen_dir: None,
            last_dump_message: None,
        };

        if let Err(e) = mgr.refresh_current_tab() {
            warn!(
                "Failed to initialize tab information for session '{}': {}",
                session_name, e
            );
        }

        debug!("Current panel: {:?}", mgr.current_tab_name());

        Ok(mgr)
    }

    pub fn set_log_dir(&mut self, path: PathBuf) -> anyhow::Result<()> {
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        debug!("Set log dir to: {}", path.display());
        self.dump_screen_dir = Some(path);
        Ok(())
    }
}
