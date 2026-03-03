use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::mcp::ZellijMcpServer;

// ============ Request Types ============

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AttachSessionRequest {
    /// Session name to attach to
    pub session_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DetachSessionRequest {
    /// Session name to detach from (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SelectSessionRequest {
    /// Session name to set as current
    pub session_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameSessionRequest {
    /// New name for the session
    pub new_name: String,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

// ============ Tool Router ============

#[tool_router(router = session_tools)]
impl ZellijMcpServer {
    /// Attach to a Zellij session by name
    #[tool]
    async fn attach_session(
        &self,
        Parameters(req): Parameters<AttachSessionRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.attach(req.session_name.clone()) {
            Ok(_) => {
                tracing::info!("Attached to '{}'", req.session_name);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Attached to session '{}' (set as current)",
                    req.session_name
                ))]))
            }
            Err(e) => {
                tracing::error!("Failed to attach: {}", e);
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed: {}",
                    e
                ))]))
            }
        }
    }

    /// Detach from a Zellij session
    #[tool]
    async fn detach_session(
        &self,
        Parameters(req): Parameters<DetachSessionRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let name = match req.session_name {
            Some(n) => n,
            None => match self.manager.get_current_session_name() {
                Some(n) => n,
                None => {
                    return Ok(CallToolResult::error(vec![Content::text(
                        "No session specified and no current session".to_string(),
                    )]));
                }
            },
        };

        match self.manager.detach(&name) {
            Ok(_) => {
                tracing::info!("Detached from '{}'", name);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Detached from '{}'",
                    name
                ))]))
            }
            Err(e) => {
                tracing::error!("Failed to detach: {}", e);
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed: {}",
                    e
                ))]))
            }
        }
    }

    /// List all available Zellij sessions
    #[tool]
    async fn list_sessions(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.list_available() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "No active sessions".to_string(),
                    )]));
                }

                let attached = self.manager.list_attached();
                let current = self.manager.get_current_session_name();

                let mut output = String::from("Available sessions:\n");
                for session in sessions {
                    let is_attached = attached.contains(&session);
                    let is_current = current.as_ref().map(|s| s == &session).unwrap_or(false);

                    let marker = if is_current {
                        " (current)"
                    } else if is_attached {
                        " (attached)"
                    } else {
                        ""
                    };

                    output.push_str(&format!("  - {}{}\n", session, marker));
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => {
                tracing::error!("Failed to list: {}", e);
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed: {}",
                    e
                ))]))
            }
        }
    }

    /// Select the current active session
    #[tool]
    async fn select_session(
        &self,
        Parameters(req): Parameters<SelectSessionRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self
            .manager
            .set_current_session(Some(req.session_name.clone()))
        {
            Ok(_) => {
                tracing::info!("Selected '{}'", req.session_name);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Current session: {}",
                    req.session_name
                ))]))
            }
            Err(e) => {
                tracing::error!("Failed to select: {}", e);
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed: {}",
                    e
                ))]))
            }
        }
    }

    /// Get the current active session name
    #[tool]
    async fn get_current_session(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.get_current_session_name() {
            Some(name) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Current: {}",
                name
            ))])),
            None => Ok(CallToolResult::success(vec![Content::text(
                "No current session".to_string(),
            )])),
        }
    }

    /// Rename a Zellij session
    #[tool]
    async fn rename_session(
        &self,
        Parameters(req): Parameters<RenameSessionRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session(req.session_name.clone()) {
            Ok(mgr) => match mgr.rename_session(req.new_name.clone()) {
                Ok(_) => {
                    tracing::info!("Renamed to '{}'", req.new_name);
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Renamed to '{}'",
                        req.new_name
                    ))]))
                }
                Err(e) => {
                    tracing::error!("Failed to rename: {}", e);
                    Ok(CallToolResult::error(vec![Content::text(format!(
                        "Failed: {}",
                        e
                    ))]))
                }
            },
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to resolve: {}",
                e
            ))])),
        }
    }
}

pub fn tool_router() -> ToolRouter<ZellijMcpServer> {
    ZellijMcpServer::session_tools()
}
