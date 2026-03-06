use std::path::PathBuf;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{manager::readwrite::DumpRange, mcp::ZellijMcpServer};

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
    /// Key to send
    ///
    /// Supported formats:
    ///   ctrl+c, ctrl+d, ctrl+z, ctrl+l  (ctrl+<key> or ^<key>)
    ///   alt+<char>
    ///   enter/return, tab, backspace/bs, esc/escape
    ///   delete/del, insert/ins, home, end, pageup/pgup, pagedown/pgdn
    ///   up, down, left, right, f1-f12
    ///   <single-char>
    pub key: String,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DumpScreenRequest {
    /// Return the last N lines. Mutually exclusive with begin/end.
    pub lines: Option<usize>,

    /// Start line, 1-indexed. Must be used with end. Mutually exclusive with lines.
    pub begin: Option<usize>,
    /// End line, 1-indexed. Must be used with begin. Mutually exclusive with lines.
    pub end: Option<usize>,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetLogDirRequest {
    /// Path to write logs to
    pub dir: PathBuf,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

// ============ Tool Router ============

#[tool_router(router = readwrite_tools)]
impl ZellijMcpServer {
    /// Write text to the current tab with optional newline.
    /// This command will return last 20 lines by default.
    #[tool]
    async fn write(
        &self,
        Parameters(req): Parameters<WriteRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let _op_guard = self.op_lock.lock().await;
        match self
            .manager
            .with_session_mut(req.session_name.clone(), |mgr| {
                mgr.write_text(&req.text, req.add_newline)
            }) {
            Ok(content) => {
                let action = if req.add_newline { "command" } else { "text" };
                let msg = format!("Sent {} to current tab\n{}", action, content);
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

    /// Write multiple commands to the current tab, each followed by newline
    #[tool]
    async fn write_multiple(
        &self,
        Parameters(req): Parameters<WriteMultipleRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let _op_guard = self.op_lock.lock().await;
        match self
            .manager
            .with_session_mut(req.session_name.clone(), |mgr| {
                mgr.write_multiple(&req.commands)
            }) {
            Ok(content) => {
                let msg = format!("Sent {} commands\n{}", req.commands.len(), content);
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

    /// Send a special key like 'enter', 'ctrl+c', 'f1'
    #[tool]
    async fn send_key(
        &self,
        Parameters(req): Parameters<SendKeyRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let _op_guard = self.op_lock.lock().await;
        match self.manager.with_session(req.session_name.clone(), |mgr| {
            mgr.send_key_string(&req.key)
        }) {
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

    /// Dump current screen content to dir or stdout.
    ///
    /// Note:
    ///  - dump to dir will also dump to stdout
    ///  - filename is not needed, by default will be datetime
    ///  - filename is used for human readability, llm should not rely on it
    #[tool]
    async fn dump_screen(
        &self,
        Parameters(req): Parameters<DumpScreenRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let _op_guard = self.op_lock.lock().await;
        let range = match (req.lines, req.begin, req.end) {
            (Some(n), _, _) => DumpRange::Last(n),
            (_, Some(b), Some(e)) => DumpRange::Range { begin: b, end: e },
            _ => DumpRange::Viewport,
        };
        match self
            .manager
            .with_session(req.session_name.clone(), |mgr| mgr.dump_screen(&range))
        {
            Ok((content, _)) => {
                tracing::info!("Screen dumped");
                Ok(CallToolResult::success(vec![Content::text(content)]))
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

    /// Set log directory, all dump screen logs will be written to this directory
    #[tool]
    async fn set_log_dir(
        &self,
        Parameters(req): Parameters<SetLogDirRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let _op_guard = self.op_lock.lock().await;
        tracing::info!(log_dir = %req.dir.display(), "Setting log dir");
        match self.manager.set_session_log_dir(req.session_name, req.dir) {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text("Log dir set")])),
            Err(e) => {
                tracing::error!("Failed to set log dir: {}", e);
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed: {}",
                    e
                ))]))
            }
        }
    }
}

pub fn tool_router() -> ToolRouter<ZellijMcpServer> {
    ZellijMcpServer::readwrite_tools()
}
