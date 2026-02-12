use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool_handler,
};

pub mod manager;
pub mod tools;

use manager::SessionManager;

/// Zellij MCP Server
#[derive(Clone)]
pub struct ZellijMcpServer {
    pub manager: SessionManager,
    tool_router: ToolRouter<Self>,
}

impl ZellijMcpServer {
    pub fn new() -> Self {
        let manager = SessionManager::default();

        // Combine all tool routers
        let tool_router = ToolRouter::new()
            + tools::session::tool_router()
            + tools::tab::tool_router()
            + tools::readwrite::tool_router();

        Self {
            manager,
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

impl Default for ZellijMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
