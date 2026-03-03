use std::collections::BTreeSet;

use anyhow::Context;
use colored::Colorize;
use zellij_utils::data::{BareKey, KeyModifier};

use crate::{
    interactive::context::CliContext,
    manager::{format_key_name, readwrite::DumpRange},
};

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
        let content = mgr.write_text(&text, add_newline)?;

        println!("{}", content);

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
        let content = mgr.write_multiple(&commands)?;

        println!("{}", content);

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
        let key_name = format_key_name(&key, &modifiers);

        mgr.send_key(key, modifiers)?;

        println!("{}", format!("✓ Sent {} to current tab", key_name).green());

        Ok(())
    }

    pub(super) fn dump_screen(context: &mut CliContext, range: &DumpRange) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        let (content, path) = mgr.dump_screen(range)?;

        match path {
            Some(ref file_path) => {
                let mode = match range {
                    DumpRange::Last(n) => format!("(last {} lines)", n),
                    DumpRange::Range { begin, end } => {
                        format!("(lines {} to {} (1-indexed))", begin, end)
                    }
                    DumpRange::Viewport => "viewport".to_string(),
                };
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
