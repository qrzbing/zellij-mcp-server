use std::str::FromStr;

use anyhow::{Result, bail};
use kdl::KdlDocument;
use tracing::{debug, warn};
use zellij_utils::{cli::CliAction, data::ClientId};

use super::ZellijSessionManager;

impl ZellijSessionManager {
    fn tab_position_from_names(tabs: &[String], tab_name: &str) -> Option<usize> {
        tabs.iter()
            .position(|name| name == tab_name)
            .map(|idx| idx + 1) // zellij tab positions are 1-based
    }

    pub(crate) fn preferred_tab_position(&self) -> Option<usize> {
        let current = self.current_tab_name.as_deref()?;
        let tabs = self.list_tabs().ok()?;
        Self::tab_position_from_names(&tabs, current)
    }

    pub fn new_tab(&mut self, name: Option<String>) -> Result<Option<String>> {
        let action = CliAction::NewTab {
            layout: None,
            layout_dir: None,
            name: name.clone(),
            cwd: None,
            initial_command: Vec::new(),
            initial_plugin: None,
            close_on_exit: false,
            start_suspended: false,
            block_until_exit_success: false,
            block_until_exit_failure: false,
            block_until_exit: false,
        };
        if self.has_active_ui_clients() {
            self.send_action(action, None)?;
        } else {
            self.send_action_as_ui_client(action, None, self.preferred_tab_position())?;
        }

        debug!("Created new tab: {:?}", name);

        self.refresh_current_tab()?;

        Ok(name)
    }

    pub fn close_tab(&mut self) -> Result<()> {
        let action = CliAction::CloseTab { tab_id: None };
        if self.has_active_ui_clients() {
            self.send_action(action, None)?;
        } else {
            self.send_action_as_ui_client(action, None, self.preferred_tab_position())?;
        }

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

    fn extract_client_ids_from_list_clients(output: &str) -> Vec<ClientId> {
        output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter_map(|line| line.split_whitespace().next())
            .filter_map(|client_id| client_id.parse::<ClientId>().ok())
            .collect()
    }

    fn list_client_ids(&self) -> Result<Vec<ClientId>> {
        let clients_str = self.send_action(CliAction::ListClients, None)?;
        Ok(Self::extract_client_ids_from_list_clients(&clients_str))
    }

    pub(crate) fn has_active_ui_clients(&self) -> bool {
        match self.list_client_ids() {
            Ok(client_ids) => !client_ids.is_empty(),
            Err(e) => {
                warn!("Failed to detect connected clients from ListClients: {}", e);
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

        // In headless mode, DumpLayout can omit focused tab. Keep the previous
        // cached hint so subsequent UI-client attach can still target the tab.
        if let Some(name) = tab_name {
            self.current_tab_name = Some(name);
        }

        debug!("Refreshed current tab: {:?}", self.current_tab_name);

        Ok(self.current_tab_name.clone())
    }

    pub fn switch_to_tab(&mut self, tab_name: String) -> Result<()> {
        let tabs = self.list_tabs()?;
        if !tabs.iter().any(|name| name == &tab_name) {
            bail!(
                "Tab '{}' not found. Available tabs: {}",
                tab_name,
                tabs.join(", ")
            );
        }

        let switch_action = CliAction::GoToTabName {
            name: tab_name.clone(),
            create: false,
        };
        if self.has_active_ui_clients() {
            self.send_action(switch_action, None)?;
            let current = self.refresh_current_tab()?;
            if current.as_deref() != Some(tab_name.as_str()) {
                let current = current.unwrap_or_else(|| "<none>".to_string());
                bail!(
                    "Switch command was sent, but focused tab is '{}' instead of '{}'",
                    current,
                    tab_name
                );
            }
        } else {
            // In headless mode there is no persistent active client; keep an internal tab hint
            // so subsequent write/dump operations can focus this tab on attach.
            let tab_position_to_focus = Self::tab_position_from_names(&tabs, &tab_name);
            self.send_action_as_ui_client(switch_action, None, tab_position_to_focus)?;
            self.current_tab_name = Some(tab_name.clone());
        }

        debug!("Switched to tab: {}", tab_name);

        Ok(())
    }

    /// Rename the current tab
    pub fn rename_tab(&mut self, new_name: String) -> Result<()> {
        let action = CliAction::RenameTab {
            name: new_name.clone(),
            tab_id: None,
        };
        if self.has_active_ui_clients() {
            self.send_action(action, None)?;
        } else {
            self.send_action_as_ui_client(action, None, self.preferred_tab_position())?;
        }

        self.current_tab_name = Some(new_name.clone());

        debug!("Renamed current tab to: {}", new_name);

        Ok(())
    }

    /// Undo tab rename (restore to default name like "Tab #1")
    pub fn undo_rename_tab(&mut self) -> Result<()> {
        let action = CliAction::UndoRenameTab { tab_id: None };
        if self.has_active_ui_clients() {
            self.send_action(action, None)?;
        } else {
            self.send_action_as_ui_client(action, None, self.preferred_tab_position())?;
        }

        self.refresh_current_tab()?;

        debug!(
            "Undone tab rename, current tab: {:?}",
            self.current_tab_name
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ZellijSessionManager;

    #[test]
    fn extract_client_ids_from_list_clients_parses_data_rows() {
        let output = "\
CLIENT_ID ZELLIJ_PANE_ID RUNNING_COMMAND
1         terminal_4     zsh
12        plugin_7       zellij:tab-bar
";

        let client_ids = ZellijSessionManager::extract_client_ids_from_list_clients(output);

        assert_eq!(client_ids, vec![1, 12]);
    }

    #[test]
    fn extract_client_ids_from_list_clients_handles_header_only() {
        let output = "CLIENT_ID ZELLIJ_PANE_ID RUNNING_COMMAND";

        let client_ids = ZellijSessionManager::extract_client_ids_from_list_clients(output);

        assert!(client_ids.is_empty());
    }
}
