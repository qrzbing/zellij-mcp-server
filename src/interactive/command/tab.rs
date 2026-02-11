use anyhow::Context;
use colored::Colorize;

use crate::interactive::context::CliContext;

use super::CommandExecutor;

impl CommandExecutor {
    pub(super) fn new_tab(context: &mut CliContext, name: Option<String>) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        mgr.new_tab(name.clone())?;
        let msg = format!("✓ Created new tab");
        println!("{}", msg.green());
        Ok(())
    }

    pub(super) fn close_tab(context: &mut CliContext) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        mgr.close_tab()?;

        println!("{}", "✓ Closed current tab".green());
        Ok(())
    }

    pub(super) fn list_tabs(context: &CliContext) -> anyhow::Result<()> {
        let mgr = context
            .manager()
            .with_context(|| "Failed to retrieve manager context")?;
        let tabs = mgr.list_tabs()?;

        if tabs.is_empty() {
            println!("  {}", "No active sessions found.".dimmed());
        } else {
            let current_tab_name = mgr.current_tab_name();
            for tab in tabs {
                if Some(tab.as_str()) == current_tab_name.as_deref() {
                    println!("  {} {}", "→".green(), tab.green().bold());
                } else {
                    println!("  - {}", tab);
                }
            }
        }

        Ok(())
    }

    pub(super) fn switch_to_tab(context: &mut CliContext, tab_name: String) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Failed to retrieve manager context")?;
        mgr.switch_to_tab(tab_name.clone())?;
        println!("Switched to tab: {}", tab_name);
        Ok(())
    }
}
