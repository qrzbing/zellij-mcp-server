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
pub struct WriteRequest {
    /// Text to write (supports escape sequences like \n, \t, \e)
    pub text: String,
    /// Whether to add newline at the end (default: true)
    #[serde(default = "default_true")]
    pub add_newline: bool,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteMultipleRequest {
    /// Multiple commands to write (each followed by newline)
    pub commands: Vec<String>,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendKeyRequest {
    /// Key to send (e.g., "enter", "tab", "ctrl+c", "f1")
    pub key: String,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DumpScreenRequest {
    /// Optional file path to save the dump (stdout if not provided)
    pub path: Option<String>,
    /// Include full scrollback history (default: false)
    #[serde(default)]
    pub full: bool,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

// ============ Tool Router ============

#[tool_router(router = readwrite_tools)]
impl ZellijMcpServer {
    /// Write text to the current tab
    #[tool(description = "Write text to the current tab with optional newline")]
    async fn write(
        &self,
        Parameters(req): Parameters<WriteRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session(req.session_name.clone()) {
            Ok(mgr) => {
                // Use Manager's high-level API (automatically processes escape sequences)
                match mgr.write_text(&req.text, req.add_newline) {
                    Ok(_) => {
                        let action = if req.add_newline { "command" } else { "text" };
                        let msg = format!("Sent {} to current tab", action);
                        tracing::info!("{}", msg);
                        Ok(CallToolResult::success(vec![Content::text(msg)]))
                    }
                    Err(e) => {
                        tracing::error!("Failed to write: {}", e);
                        Ok(CallToolResult::error(vec![Content::text(format!(
                            "Failed to write: {}",
                            e
                        ))]))
                    }
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to resolve session: {}",
                e
            ))])),
        }
    }

    /// Write multiple commands to the current tab
    #[tool(description = "Write multiple commands, each followed by newline")]
    async fn write_multiple(
        &self,
        Parameters(req): Parameters<WriteMultipleRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session(req.session_name.clone()) {
            Ok(mgr) => {
                // Use Manager's high-level API
                match mgr.write_multiple(&req.commands) {
                    Ok(_) => {
                        let msg = format!("Sent {} commands", req.commands.len());
                        tracing::info!("{}", msg);
                        Ok(CallToolResult::success(vec![Content::text(msg)]))
                    }
                    Err(e) => {
                        tracing::error!("Failed to write commands: {}", e);
                        Ok(CallToolResult::error(vec![Content::text(format!(
                            "Failed: {}",
                            e
                        ))]))
                    }
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to resolve session: {}",
                e
            ))])),
        }
    }

    /// Send a special key to the current tab
    #[tool(description = "Send a special key like 'enter', 'ctrl+c', 'f1'")]
    async fn send_key(
        &self,
        Parameters(req): Parameters<SendKeyRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session(req.session_name.clone()) {
            Ok(mgr) => {
                // Use Manager's high-level API (automatically parses key string)
                match mgr.send_key_string(&req.key) {
                    Ok(key_name) => {
                        let msg = format!("Sent key: {}", key_name);
                        tracing::info!("{}", msg);
                        Ok(CallToolResult::success(vec![Content::text(msg)]))
                    }
                    Err(e) => {
                        tracing::error!("Failed to send key '{}': {}", req.key, e);
                        Ok(CallToolResult::error(vec![Content::text(format!(
                            "Failed to parse or send key: {}",
                            e
                        ))]))
                    }
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to resolve session: {}",
                e
            ))])),
        }
    }

    /// Dump the current screen content
    #[tool(description = "Dump screen content to file or stdout")]
    async fn dump_screen(
        &self,
        Parameters(req): Parameters<DumpScreenRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session(req.session_name.clone()) {
            Ok(mgr) => {
                // Use Manager's API
                match mgr.dump_screen(req.path.clone(), req.full) {
                    Ok(content) => {
                        let msg = match req.path {
                            Some(ref path) => format!("Dumped to: {}", path),
                            None => content,
                        };
                        tracing::info!("Screen dumped");
                        Ok(CallToolResult::success(vec![Content::text(msg)]))
                    }
                    Err(e) => {
                        tracing::error!("Failed to dump screen: {}", e);
                        Ok(CallToolResult::error(vec![Content::text(format!(
                            "Failed: {}",
                            e
                        ))]))
                    }
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to resolve session: {}",
                e
            ))])),
        }
    }
}

pub fn tool_router() -> ToolRouter<ZellijMcpServer> {
    ZellijMcpServer::readwrite_tools()
}
