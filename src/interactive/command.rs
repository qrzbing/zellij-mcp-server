use anyhow::Ok;
use colored::Colorize;

use crate::{interactive::context::CliContext, manager::session::ZellijSessionManager};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // Session Commands
    Attach { session_name: String },
    Detach,
    ListSessions,
    //
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

        let parts: Vec<&str> = input.split_whitespace().collect();
        let cmd = parts[0];

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
            "list-sessions" | "ls" => Ok(Command::ListSessions),
            //
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
            Command::ListSessions => {
                Self::list_sessions(context)?;
                Ok(false)
            }

            // Other Commands
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
