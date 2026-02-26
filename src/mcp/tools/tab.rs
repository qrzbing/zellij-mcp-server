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
pub struct NewTabRequest {
    /// Optional name for the new tab
    pub name: Option<String>,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CloseTabRequest {
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTabsRequest {
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SwitchTabRequest {
    /// Tab name to switch to
    pub tab_name: String,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameTabRequest {
    /// New name for the tab (None to undo rename)
    pub new_name: Option<String>,
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowLayoutRequest {
    /// Session name (uses current if not provided)
    pub session_name: Option<String>,
}

// ============ Tool Router ============

#[tool_router(router = tab_tools)]
impl ZellijMcpServer {
    /// Create a new tab
    #[tool]
    async fn new_tab(
        &self,
        Parameters(req): Parameters<NewTabRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session_mut(req.session_name) {
            Ok(mut mgr) => match mgr.new_tab(req.name.clone()) {
                Ok(_) => {
                    let msg = req
                        .name
                        .map(|n| format!("Created tab '{}'", n))
                        .unwrap_or_else(|| "Created new tab".to_string());
                    tracing::info!("{}", msg);
                    Ok(CallToolResult::success(vec![Content::text(msg)]))
                }
                Err(e) => {
                    tracing::error!("Failed to create tab: {}", e);
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

    /// Close current tab
    #[tool]
    async fn close_tab(
        &self,
        Parameters(req): Parameters<CloseTabRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session_mut(req.session_name) {
            Ok(mut mgr) => match mgr.close_tab() {
                Ok(_) => {
                    tracing::info!("Closed tab");
                    Ok(CallToolResult::success(vec![Content::text(
                        "Closed tab".to_string(),
                    )]))
                }
                Err(e) => {
                    tracing::error!("Failed to close: {}", e);
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

    /// List all tabs
    #[tool]
    async fn list_tabs(
        &self,
        Parameters(req): Parameters<ListTabsRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session(req.session_name) {
            Ok(mgr) => match mgr.list_tabs() {
                Ok(tabs) => {
                    if tabs.is_empty() {
                        return Ok(CallToolResult::success(vec![Content::text(
                            "No tabs".to_string(),
                        )]));
                    }

                    let current = mgr.current_tab_name();
                    let mut output = String::from("Tabs:\n");
                    for tab in tabs {
                        let marker = if current == Some(tab.as_str()) {
                            " (current)"
                        } else {
                            ""
                        };
                        output.push_str(&format!("  - {}{}\n", tab, marker));
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
            },
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to resolve: {}",
                e
            ))])),
        }
    }

    /// Switch tab
    #[tool(description = "Switch to a specific tab")]
    async fn switch_tab(
        &self,
        Parameters(req): Parameters<SwitchTabRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session_mut(req.session_name) {
            Ok(mut mgr) => match mgr.switch_to_tab(req.tab_name.clone()) {
                Ok(_) => {
                    tracing::info!("Switched to '{}'", req.tab_name);
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Switched to '{}'",
                        req.tab_name
                    ))]))
                }
                Err(e) => {
                    tracing::error!("Failed to switch: {}", e);
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

    /// Rename tab
    #[tool(description = "Rename the current tab")]
    async fn rename_tab(
        &self,
        Parameters(req): Parameters<RenameTabRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session_mut(req.session_name) {
            Ok(mut mgr) => {
                let result = if let Some(name) = req.new_name {
                    mgr.rename_tab(name.clone())
                        .map(|_| format!("Renamed to '{}'", name))
                } else {
                    mgr.undo_rename_tab()
                        .map(|_| "Restored default name".to_string())
                };

                match result {
                    Ok(msg) => {
                        tracing::info!("{}", msg);
                        Ok(CallToolResult::success(vec![Content::text(msg)]))
                    }
                    Err(e) => {
                        tracing::error!("Failed to rename: {}", e);
                        Ok(CallToolResult::error(vec![Content::text(format!(
                            "Failed: {}",
                            e
                        ))]))
                    }
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to resolve: {}",
                e
            ))])),
        }
    }

    /// Show layout
    #[tool(description = "Show session layout")]
    async fn show_layout(
        &self,
        Parameters(req): Parameters<ShowLayoutRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.manager.resolve_session(req.session_name) {
            Ok(mgr) => match mgr.send_action(zellij_utils::cli::CliAction::DumpLayout, None) {
                Ok(layout) => {
                    tracing::info!("Retrieved layout");
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Layout:\n{}",
                        layout
                    ))]))
                }
                Err(e) => {
                    tracing::error!("Failed to get layout: {}", e);
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
    ZellijMcpServer::tab_tools()
}
