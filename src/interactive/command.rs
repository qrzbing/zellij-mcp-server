use anyhow::{Context, Ok};
use colored::Colorize;
use shlex;
use zellij_utils::cli::CliAction;

use crate::{interactive::context::CliContext, manager::ZellijSessionManager};

#[derive(Debug, Clone, PartialEq)]
pub enum RenameTarget {
    Tab,
    Session,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // Session Commands
    Attach {
        session_name: String,
    },
    Detach,
    List {
        filter: String,
    },
    // Layout Commands
    ShowLayout,
    Switch {
        tab_name: String,
    },
    // Other Commands
    Rename {
        target: RenameTarget,
        name: Option<String>,
    },
    Status,
    Help,
    Exit,
}

pub struct CommandParser;

impl CommandParser {
    pub fn parse(input: &str) -> anyhow::Result<Command> {
        let input = input.trim();

        if input.is_empty() {
            anyhow::bail!("Empty command");
        }

        let parts: Vec<String> =
            shlex::split(input).ok_or_else(|| anyhow::anyhow!("Failed to parse command"))?;
        if parts.is_empty() {
            anyhow::bail!("Empty command");
        }

        let cmd = parts[0].as_str();

        match cmd {
            // Session Commands
            "attach" | "a" => {
                if parts.len() < 2 {
                    anyhow::bail!("Usage: attach <session-name>");
                }
                Ok(Command::Attach {
                    session_name: parts[1].to_string(),
                })
            }
            "detach" | "d" => Ok(Command::Detach),
            "list" | "ls" => {
                if parts.len() < 2 {
                    anyhow::bail!("Usage: ls [session | tab]");
                }
                Ok(Command::List {
                    filter: parts[1].to_string(),
                })
            }
            // Tab Commands
            "layout" | "show-layout" => Ok(Command::ShowLayout),
            "switch" | "s" => {
                if parts.len() < 2 {
                    anyhow::bail!("Usage: switch <tab-name>");
                }
                Ok(Command::Switch {
                    tab_name: parts[1].to_string(),
                })
            }
            // Other Commands
            "rename" | "r" => {
                if parts.len() < 2 {
                    anyhow::bail!("Usage: rename <tab|session> [name] [--undo|-u]");
                }

                let target = match parts[1].as_str() {
                    "tab" | "t" => RenameTarget::Tab,
                    "session" | "s" => RenameTarget::Session,
                    _ => anyhow::bail!(
                        "Unknown rename target: '{}'. Use 'tab' or 'session'.",
                        parts[1]
                    ),
                };

                let is_undo = parts.iter().any(|p| p == "--undo" || p == "-u");

                let name = if is_undo {
                    None
                } else if parts.len() < 3 {
                    anyhow::bail!("Usage: rename {} <name>", parts[1]);
                } else {
                    Some(parts[2].clone())
                };

                Ok(Command::Rename { target, name })
            }
            "status" => Ok(Command::Status),
            "help" | "h" | "?" => Ok(Command::Help),
            "exit" | "quit" | "q" => Ok(Command::Exit),

            _ => anyhow::bail!(
                "Unknown command: '{}'. Type 'help' for available commands.",
                cmd
            ),
        }
    }
}

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute(command: Command, context: &mut CliContext) -> anyhow::Result<bool> {
        match command {
            // Session Commands
            Command::Attach { session_name } => {
                Self::attach_session(session_name, context)?;
                Ok(false)
            }
            Command::Detach => {
                Self::detach_session(context);
                Ok(false)
            }
            Command::List { filter } => {
                match filter.as_str() {
                    "session" | "s" => {
                        Self::list_sessions(context)?;
                    }
                    "tab" | "t" => {
                        Self::list_tabs(context)?;
                    }
                    _ => {
                        println!("Unknown filter: {}", filter);
                    }
                }
                Ok(false)
            }
            // Layout Commands
            Command::ShowLayout => {
                Self::show_layout(context)?;
                Ok(false)
            }
            Command::Switch { tab_name } => {
                Self::switch_to_tab(context, tab_name.clone())?;
                Ok(false)
            }
            // Other Commands
            Command::Rename { target, name } => {
                Self::rename(context, target, name)?;
                Ok(false)
            }
            Command::Status => {
                Self::show_status(context);
                Ok(false)
            }

            Command::Help => {
                Self::show_help();
                Ok(false)
            }

            Command::Exit => {
                println!("{}", "Goodbye!".green());
                Ok(true)
            }
        }
    }

    fn attach_session(session_name: String, context: &mut CliContext) -> anyhow::Result<()> {
        context.attach(session_name.clone())?;
        println!(
            "{} {}",
            "✓ Attached to session:".green(),
            session_name.bold()
        );
        Ok(())
    }

    fn detach_session(context: &mut CliContext) {
        context.detach();
        println!("{}", "✓ Detached from session.".green());
    }

    fn list_sessions(context: &CliContext) -> anyhow::Result<()> {
        let sessions = ZellijSessionManager::list_sessions(&context.socket_dir)?;

        if sessions.is_empty() {
            println!("  {}", "No active sessions found.".dimmed());
        } else {
            let current_session_name = context.current_session_name();
            for session in sessions {
                if Some(session.as_str()) == current_session_name.as_deref() {
                    println!("  {} {}", "→".green(), session.green().bold());
                } else {
                    println!("  - {}", session);
                }
            }
        }

        Ok(())
    }

    fn list_tabs(context: &CliContext) -> anyhow::Result<()> {
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

    fn switch_to_tab(context: &mut CliContext, tab_name: String) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Failed to retrieve manager context")?;
        mgr.switch_to_tab(tab_name.clone())?;
        println!("Switched to tab: {}", tab_name);
        Ok(())
    }

    fn rename(
        context: &mut CliContext,
        target: RenameTarget,
        name: Option<String>,
    ) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        match target {
            RenameTarget::Tab => {
                if let Some(name) = name {
                    mgr.rename_tab(name.clone())?;
                    println!("✓ Renamed current tab to: {}", name);
                } else {
                    mgr.undo_rename_tab()?;
                    println!("✓ Restored tab to default name");
                }
            }
            RenameTarget::Session => {
                if let Some(name) = name {
                    mgr.rename_session(name.clone())?;
                    println!("✓ Renamed session to: {}", name);
                } else {
                    anyhow::bail!("Cannot undo session rename. Please provide a new name.");
                }
            }
        }

        Ok(())
    }

    fn show_layout(context: &CliContext) -> anyhow::Result<()> {
        if !context.is_attached() {
            anyhow::bail!("Not attached to any session");
        }

        let manager = context
            .manager()
            .with_context(|| "Failed to retrieve manager context")?;
        let layout = manager.send_action(CliAction::DumpLayout, None)?;

        println!("{}", "Layout:".bold());
        println!("{}", layout);

        Ok(())
    }

    fn show_status(context: &CliContext) {
        println!("{}", "Status:".bold());
        println!("zellij path: {}", context.zellij_path);
        println!("socket dir: {}", context.socket_dir.display());

        let session_name = match context.current_session_name() {
            Some(session_name) => session_name.green(),
            None => "None".red(),
        };

        println!("current session: {}", session_name);
    }

    fn show_help() {
        println!("{}", include_str!("../docs/cli-help.md"));
    }
}
