use std::str::FromStr;

use anyhow::Result;
use kdl::KdlDocument;
use tracing::debug;
use zellij_utils::cli::CliAction;

use super::ZellijSessionManager;

impl ZellijSessionManager {
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
        self.send_action(
            CliAction::GoToTabName {
                name: tab_name.clone(),
                create: false,
            },
            None,
        )?;

        // Update cache
        self.current_tab_name = Some(tab_name.clone());

        debug!("Switched to tab: {}", tab_name);

        Ok(())
    }
}
