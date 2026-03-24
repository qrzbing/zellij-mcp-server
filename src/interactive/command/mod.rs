use std::path::PathBuf;

use anyhow::{Context, Ok};
use clap::{Parser, Subcommand};
use colored::Colorize;
use zellij_utils::cli::CliAction;

use crate::{
    interactive::context::CliContext,
    manager::{self, readwrite::DumpRange},
};

mod readwrite;
mod session;
mod tab;

#[derive(Subcommand, Debug, Clone)]
pub enum RenameTarget {
    /// Rename current tab
    #[command(alias = "t")]
    Tab {
        /// New name for the target
        name: Option<String>,

        /// Undo previous rename
        #[arg(short = 'u', long)]
        undo: bool,
    },

    /// Rename current session
    #[command(alias = "s")]
    Session {
        /// New name for the session
        #[arg(required = true)]
        name: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum DumpCommand {
    /// Dump current screen
    #[command(alias = "s")]
    Screen {
        /// Return the last N lines
        #[arg(short = 'n', long, conflicts_with_all = ["begin", "end"])]
        lines: Option<usize>,

        /// Start line (1-indexed, must be used with --end)
        #[arg(long, conflicts_with = "lines", requires = "end")]
        begin: Option<usize>,

        /// End line (1-indexed, must be used with --begin)
        #[arg(long, conflicts_with = "lines", requires = "begin")]
        end: Option<usize>,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SetCommand {
    /// If set LogDir, all dump screen commands will log to LogDir
    LogDir {
        /// Path to the LogDir
        path: PathBuf,
    },
}

#[derive(Parser, Debug)]
#[command(multicall = true)]
pub enum ReplCommand {
    /// Attach to an existing zellij session
    #[command(alias = "a")]
    Attach {
        /// Session name to attach to
        session_name: String,
    },

    /// Close current tab
    CloseTab,

    /// Detach from the current session
    #[command(alias = "d")]
    Detach,

    /// Dump the current screen
    Dump {
        #[command(subcommand)]
        dump_command: DumpCommand,
    },

    /// Exit the interactive shell
    #[command(alias = "q", alias = "quit")]
    Exit,

    /// List active sessions or tabs
    #[command(alias = "ls")]
    List {
        /// Filter: "session" (or "s") | "tab" (or "t")
        filter: String,
    },

    /// Create a new tab
    #[command(alias = "n", name = "new")]
    NewTab {
        /// Optional tab name
        name: Option<String>,
    },

    /// Create a new detached zellij session and attach to it
    #[command(alias = "ns", name = "new-session")]
    NewSession {
        /// Session name to create
        session_name: String,
    },

    /// Switch to a specific tab
    #[command(alias = "s")]
    Switch {
        /// Name of the tab to switch to
        tab_name: String,
    },

    /// Rename current tab or session
    #[command(alias = "r")]
    Rename {
        #[command(subcommand)]
        target: RenameTarget,
    },

    /// Send a specific key to the current tab
    Send {
        /// Key to send
        ///
        /// Supported formats:
        ///   ctrl+c, ctrl+d, ctrl+z, ctrl+l  (ctrl+<key> or ^<key>)
        ///   alt+<char>
        ///   enter/return, tab, backspace/bs, esc/escape
        ///   delete/del, insert/ins, home, end, pageup/pgup, pagedown/pgdn
        ///   up, down, left, right, f1-f12
        ///   <single-char>
        #[arg(value_name = "KEY")]
        key: String,
    },

    /// Set some config
    Set {
        #[command(subcommand)]
        set_command: SetCommand,
    },

    /// Show current layout
    #[command(alias = "layout", name = "show-layout")]
    ShowLayout,

    /// Show MCP status
    Status,

    /// Write text to the current tab
    #[command(alias = "w")]
    Write {
        /// Do not add a newline at the end
        #[arg(short = 'n', long = "no-enter")]
        no_enter: bool,

        /// The text to write (multiple texts will be executed sequentially)
        #[arg(required = true)]
        texts: Vec<String>,
    },
}

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute(command: ReplCommand, context: &mut CliContext) -> anyhow::Result<bool> {
        match command {
            // Session Commands
            ReplCommand::Attach { session_name } => {
                Self::attach_session(session_name, context)?;
                Ok(false)
            }
            ReplCommand::CloseTab => {
                Self::close_tab(context)?;
                Ok(false)
            }
            ReplCommand::Detach => {
                Self::detach_session(context);
                Ok(false)
            }
            ReplCommand::Dump { dump_command } => {
                match dump_command {
                    DumpCommand::Screen { lines, begin, end } => {
                        let range = match (lines, begin, end) {
                            (Some(n), _, _) => DumpRange::Last(n),
                            (_, Some(b), Some(e)) => DumpRange::Range { begin: b, end: e },
                            _ => DumpRange::Viewport,
                        };
                        Self::dump_screen(context, &range)?;
                    }
                }
                Ok(false)
            }
            ReplCommand::Exit => {
                println!("{}", "Goodbye!".green());
                Ok(true)
            }
            ReplCommand::List { filter } => {
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
            ReplCommand::NewTab { name } => {
                Self::new_tab(context, name.clone())?;
                Ok(false)
            }
            ReplCommand::NewSession { session_name } => {
                Self::new_session(session_name, context)?;
                Ok(false)
            }
            ReplCommand::Rename { target } => {
                Self::rename(context, target)?;
                Ok(false)
            }
            ReplCommand::Send { key } => {
                let (bare_key, modifiers) = manager::parse_key_string(&key)?;
                Self::send_key_to_tab(context, bare_key, modifiers)?;
                Ok(false)
            }
            ReplCommand::Set { set_command } => {
                match set_command {
                    SetCommand::LogDir { path } => {
                        Self::set_log_dir(context, path.clone())?;
                    }
                }
                Ok(false)
            }
            ReplCommand::ShowLayout => {
                Self::show_layout(context)?;
                Ok(false)
            }
            ReplCommand::Status => {
                Self::show_status(context);
                Ok(false)
            }
            ReplCommand::Switch { tab_name } => {
                Self::switch_to_tab(context, tab_name.clone())?;
                Ok(false)
            }
            ReplCommand::Write { no_enter, texts } => {
                if texts.len() > 1 {
                    Self::write_multiple_to_tab(context, texts)?;
                } else {
                    Self::write_to_tab(context, texts[0].clone(), !no_enter)?;
                }
                Ok(false)
            }
        }
    }

    fn rename(context: &mut CliContext, target: RenameTarget) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        match target {
            RenameTarget::Tab { name, undo } => {
                if undo {
                    mgr.undo_rename_tab()?;
                    println!("✓ Restored tab to default name");
                    return Ok(());
                }
                if let Some(name) = name {
                    mgr.rename_tab(name.clone())?;
                    println!("✓ Renamed current tab to: {}", name);
                } else {
                    mgr.undo_rename_tab()?;
                    println!("✓ Restored tab to default name");
                }
            }
            RenameTarget::Session { name } => {
                mgr.rename_session(name.clone())?;
                println!("✓ Renamed session to: {}", name);
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
        println!("zellij path: {}", context.zellij_path.display());
        println!("socket dir: {}", context.socket_dir.display());

        let session_name = match context.current_session_name() {
            Some(session_name) => session_name.green(),
            None => "None".red(),
        };

        println!("current session: {}", session_name);
    }

    fn set_log_dir(context: &mut CliContext, path: PathBuf) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        mgr.set_log_dir(path.clone())?;
        Ok(())
    }
}
