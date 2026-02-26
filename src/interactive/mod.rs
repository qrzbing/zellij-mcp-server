use std::path::PathBuf;

use colored::Colorize;
use rustyline::{DefaultEditor, error::ReadlineError};

mod command;
mod context;

use command::{CommandExecutor, CommandParser};
use context::CliContext;

pub struct InteractiveCli<'a> {
    editor: DefaultEditor,
    context: CliContext<'a>,
}

impl<'a> InteractiveCli<'a> {
    pub fn new(socket_dir: &'a PathBuf, zellij_path: &'a PathBuf) -> anyhow::Result<Self> {
        let mut editor = DefaultEditor::new()?;

        let history_file = dirs::home_dir()
            .unwrap_or_default()
            .join(".zellij_mcp_history");

        let _ = editor.load_history(&history_file);

        let context = CliContext::new(socket_dir, zellij_path);

        Ok(Self { editor, context })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        self.show_banner();

        loop {
            let prompt = self.build_prompt();

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    // Add line to history
                    let _ = self.editor.add_history_entry(line.as_str());

                    match CommandParser::parse(&line) {
                        Ok(command) => match CommandExecutor::execute(command, &mut self.context) {
                            Ok(should_exit) => {
                                if should_exit {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("{} {}", "Error:".red().bold(), e);
                            }
                        },
                        Err(e) => {
                            if !line.trim().is_empty() {
                                eprintln!("{} {}", "Error:".red().bold(), e);
                            }
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C
                    println!("{}", "(Ctrl+D to exit)".dimmed());
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D
                    println!("{}", "Goodbye!".green());
                    break;
                }
                Err(err) => {
                    eprintln!("{} {:?}", "Error:".red().bold(), err);
                    break;
                }
            }
        }

        let history_file = dirs::home_dir()
            .unwrap_or_default()
            .join(".zellij_mcp_history");

        let _ = self.editor.save_history(&history_file);

        Ok(())
    }

    fn show_banner(&self) {
        println!("{}", "=".repeat(60).dimmed());
        println!(
            "{}",
            format!("Zellij MCP Interactive CLI v{}", env!("CARGO_PKG_VERSION"))
                .bold()
                .cyan()
        );
        println!("{}", "=".repeat(60).dimmed());
        println!(
            "Type {} for available commands, {} to quit",
            "'help'".cyan(),
            "'exit'".cyan()
        );
        println!();
    }

    fn build_prompt(&self) -> String {
        if let Some(session) = self.context.current_session_name() {
            format!("{} >>> ", format!("[{}]", session).green().bold())
        } else {
            ">>> ".to_string()
        }
    }
}
