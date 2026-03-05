use std::str::FromStr;

use anyhow::Result;
use kdl::KdlDocument;
use tracing::{debug, warn};
use zellij_utils::cli::CliAction;

use super::ZellijSessionManager;

impl ZellijSessionManager {
    pub fn new_tab(&mut self, name: Option<String>) -> Result<Option<String>> {
        self.send_action(
            CliAction::NewTab {
                name: name.clone(),
                cwd: None,
                layout: None,
                layout_dir: None,
            },
            None,
        )?;

        debug!("Created new tab: {:?}", name);

        self.refresh_current_tab()?;

        Ok(name)
    }

    pub fn close_tab(&mut self) -> Result<()> {
        self.send_action(CliAction::CloseTab, None)?;

        debug!("Closed current tab");

        let _ = self.refresh_current_tab();

        Ok(())
    }

    pub fn current_tab_name(&self) -> Option<&str> {
        self.current_tab_name.as_deref()
    }

    fn extract_focused_tab_kdl(layout: &str) -> Result<Option<String>> {
        let document = KdlDocument::from_str(layout)
            .map_err(|e| anyhow::anyhow!("Failed to parse KDL layout: {}", e))?;

        let layout_node = document
            .get("layout")
            .ok_or_else(|| anyhow::anyhow!("No 'layout' node found in KDL document"))?;

        let Some(children) = layout_node.children() else {
            return Ok(None);
        };

        for node in children.nodes() {
            if node.name().value() != "tab" {
                continue;
            }

            if node.get("focus").and_then(|f| f.value().as_bool()) != Some(true) {
                continue;
            }

            if let Some(name) = node.get("name").and_then(|n| n.value().as_string()) {
                return Ok(Some(name.to_string()));
            }
        }

        Ok(None)
    }

    fn extract_connected_clients_kdl(layout: &str) -> Result<Option<usize>> {
        let document = KdlDocument::from_str(layout)
            .map_err(|e| anyhow::anyhow!("Failed to parse KDL layout: {}", e))?;

        let Some(node) = document.get("connected_clients") else {
            return Ok(None);
        };

        let count = node
            .entries()
            .iter()
            .find_map(|entry| entry.value().as_i64())
            .and_then(|v| usize::try_from(v).ok());

        Ok(count)
    }

    pub(crate) fn has_active_ui_clients(&self) -> bool {
        let layout = match self.send_action(CliAction::DumpLayout, None) {
            Ok(layout) => layout,
            Err(e) => {
                warn!("Failed to detect connected clients from DumpLayout: {}", e);
                return false;
            }
        };

        match Self::extract_connected_clients_kdl(&layout) {
            Ok(Some(count)) => count > 0,
            Ok(None) => false,
            Err(e) => {
                warn!("Failed to parse connected_clients from DumpLayout: {}", e);
                false
            }
        }
    }

    pub fn list_tabs(&self) -> Result<Vec<String>> {
        let tab_names_str = self.send_action(CliAction::QueryTabNames, None)?;
        let tab_names: Vec<String> = tab_names_str
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(tab_names)
    }

    pub fn refresh_current_tab(&mut self) -> Result<Option<String>> {
        let layout = self.send_action(CliAction::DumpLayout, None)?;
        let tab_name = Self::extract_focused_tab_kdl(&layout)?;

        // Update cache
        self.current_tab_name = tab_name.clone();

        debug!("Refreshed current tab: {:?}", self.current_tab_name);

        Ok(tab_name)
    }

    pub fn switch_to_tab(&mut self, tab_name: String) -> Result<()> {
        let switch_action = CliAction::GoToTabName {
            name: tab_name.clone(),
            create: false,
        };
        if self.has_active_ui_clients() {
            self.send_action(switch_action, None)?;
        } else {
            self.send_action_as_ui_client(switch_action, None)?;
        }

        // Update cache
        self.current_tab_name = Some(tab_name.clone());

        debug!("Switched to tab: {}", tab_name);

        Ok(())
    }

    /// Rename the current tab
    pub fn rename_tab(&mut self, new_name: String) -> Result<()> {
        self.send_action(
            CliAction::RenameTab {
                name: new_name.clone(),
            },
            None,
        )?;

        self.current_tab_name = Some(new_name.clone());

        debug!("Renamed current tab to: {}", new_name);

        Ok(())
    }

    /// Undo tab rename (restore to default name like "Tab #1")
    pub fn undo_rename_tab(&mut self) -> Result<()> {
        self.send_action(CliAction::UndoRenameTab, None)?;

        self.refresh_current_tab()?;

        debug!(
            "Undone tab rename, current tab: {:?}",
            self.current_tab_name
        );

        Ok(())
    }
}
