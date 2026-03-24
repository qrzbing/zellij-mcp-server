use colored::Colorize;

use crate::{interactive::context::CliContext, manager::ZellijSessionManager};

use super::CommandExecutor;

impl CommandExecutor {
    pub(super) fn new_session(
        session_name: String,
        context: &mut CliContext,
    ) -> anyhow::Result<()> {
        let created = context.new_session(session_name.clone())?;
        let action = if created {
            "Created and attached to session:"
        } else {
            "Attached to existing session:"
        };

        println!("{} {}", action.green(), session_name.bold());
        Ok(())
    }

    pub(super) fn attach_session(
        session_name: String,
        context: &mut CliContext,
    ) -> anyhow::Result<()> {
        context.attach(session_name.clone())?;
        println!(
            "{} {}",
            "✓ Attached to session:".green(),
            session_name.bold()
        );
        Ok(())
    }

    pub(super) fn detach_session(context: &mut CliContext) {
        context.detach();
        println!("{}", "✓ Detached from session.".green());
    }

    pub(super) fn list_sessions(context: &CliContext) -> anyhow::Result<()> {
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
}
