use std::collections::BTreeSet;

use anyhow::Context;
use colored::Colorize;
use zellij_utils::data::{BareKey, KeyModifier};

use crate::interactive::context::CliContext;
use crate::manager;

use super::CommandExecutor;

impl CommandExecutor {
    pub(super) fn write_to_tab(
        context: &mut CliContext,
        text: String,
        add_newline: bool,
    ) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        // Use Manager's high-level API
        mgr.write_text(&text, add_newline)?;

        let action = if add_newline { "command" } else { "text" };
        println!("{}", format!("✓ Sent {} to current tab", action).green());

        Ok(())
    }

    pub(super) fn write_multiple_to_tab(
        context: &mut CliContext,
        commands: Vec<String>,
    ) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        // Use Manager's high-level API
        mgr.write_multiple(&commands)?;

        println!(
            "{}",
            format!("✓ Sent {} commands to current tab", commands.len()).green()
        );

        Ok(())
    }

    pub(super) fn send_key_to_tab(
        context: &mut CliContext,
        key: BareKey,
        modifiers: BTreeSet<KeyModifier>,
    ) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        // Use Manager's utility function for formatting
        let key_name = manager::format_key_name(&key, &modifiers);

        mgr.send_key(key, modifiers)?;

        println!("{}", format!("✓ Sent {} to current tab", key_name).green());

        Ok(())
    }

    pub(super) fn dump_screen(context: &mut CliContext, full: bool) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        let (content, path) = mgr.dump_screen(full)?;

        match path {
            Some(ref file_path) => {
                let mode = if full { "with full scrollback" } else { "" };
                println!(
                    "{}",
                    format!("✓ Dumped screen {} to: {}", mode, file_path.display()).green()
                );
            }
            None => {
                println!("{}", content);
            }
        }

        Ok(())
    }
}
