use std::sync::Arc;

use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool_handler,
};
use tokio::sync::Mutex as AsyncMutex;

pub mod manager;
pub mod tools;

use crate::cli::ZellijConfig;
use manager::SessionManager;

/// Zellij MCP Server
#[derive(Clone)]
pub struct ZellijMcpServer {
    pub manager: SessionManager,
    pub op_lock: Arc<AsyncMutex<()>>,
    tool_router: ToolRouter<Self>,
}

impl ZellijMcpServer {
    pub fn new(config: ZellijConfig) -> Self {
        let manager = SessionManager::new(config.resolve_socket_path().clone());

        // Combine all tool routers
        let tool_router = ToolRouter::new()
            + tools::session::tool_router()
            + tools::tab::tool_router()
            + tools::readwrite::tool_router();

        Self {
            manager,
            op_lock: Arc::new(AsyncMutex::new(())),
            tool_router,
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ZellijMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "zellij-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                title: None,
                website_url: None,
            },
            instructions: Some(include_str!("./instructions.md").to_string()),
        }
    }
}
