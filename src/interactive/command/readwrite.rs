use std::collections::BTreeSet;

use anyhow::Context;
use colored::Colorize;
use zellij_utils::data::{BareKey, KeyModifier};

use crate::interactive::context::CliContext;

use super::{CommandExecutor, CommandParser};

fn format_key_name(key: &BareKey, modifiers: &BTreeSet<KeyModifier>) -> String {
    let mut parts = Vec::new();

    for modifier in modifiers {
        parts.push(match modifier {
            KeyModifier::Ctrl => "Ctrl",
            KeyModifier::Alt => "Alt",
            KeyModifier::Shift => "Shift",
            KeyModifier::Super => "Super",
        });
    }

    let key_str = match key {
        BareKey::Char(c) => c.to_string(),
        BareKey::Enter => "Enter".to_string(),
        BareKey::Tab => "Tab".to_string(),
        BareKey::Backspace => "Backspace".to_string(),
        BareKey::Esc => "Esc".to_string(),
        BareKey::Delete => "Delete".to_string(),
        BareKey::Insert => "Insert".to_string(),
        BareKey::Home => "Home".to_string(),
        BareKey::End => "End".to_string(),
        BareKey::PageUp => "PageUp".to_string(),
        BareKey::PageDown => "PageDown".to_string(),
        BareKey::Up => "Up".to_string(),
        BareKey::Down => "Down".to_string(),
        BareKey::Left => "Left".to_string(),
        BareKey::Right => "Right".to_string(),
        BareKey::F(n) => format!("F{}", n),
        _ => format!("{:?}", key),
    };

    if parts.is_empty() {
        key_str
    } else {
        format!("{}+{}", parts.join("+"), key_str)
    }
}

fn process_escape_sequences(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'r' => result.push('\r'),   // CR (0x0D)
                    'n' => result.push('\n'),   // LF (0x0A)
                    't' => result.push('\t'),   // Tab (0x09)
                    '0' => result.push('\0'),   // Null (0x00)
                    'e' => result.push('\x1B'), // ESC (0x1B)
                    '\\' => result.push('\\'),  // Backslash
                    'x' => {
                        let hex: String = chars.by_ref().take(2).collect();
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            result.push(byte as char);
                        } else {
                            result.push('\\');
                            result.push('x');
                            result.push_str(&hex);
                        }
                    }
                    _ => {
                        result.push('\\');
                        result.push(next);
                    }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

impl CommandParser {
    pub(super) fn parse_key(key_str: &str) -> anyhow::Result<(BareKey, BTreeSet<KeyModifier>)> {
        let mut modifiers = BTreeSet::new();
        let key_str = key_str.to_lowercase();

        if key_str.starts_with("ctrl+") || key_str.starts_with("^") {
            modifiers.insert(KeyModifier::Ctrl);
            let base = if key_str.starts_with("^") {
                &key_str[1..]
            } else {
                &key_str[5..]
            };

            let bare_key = match base {
                "c" => BareKey::Char('c'),
                "d" => BareKey::Char('d'),
                "z" => BareKey::Char('z'),
                "l" => BareKey::Char('l'),
                _ => anyhow::bail!("Unknown ctrl key: {}", base),
            };

            return Ok((bare_key, modifiers));
        }

        if key_str.starts_with("alt+") {
            modifiers.insert(KeyModifier::Alt);
            let base = &key_str[4..];
            let bare_key = BareKey::Char(base.chars().next().unwrap());
            return Ok((bare_key, modifiers));
        }

        let bare_key = match key_str.as_str() {
            "enter" | "return" => BareKey::Enter,
            "tab" => BareKey::Tab,
            "backspace" | "bs" => BareKey::Backspace,
            "esc" | "escape" => BareKey::Esc,
            "delete" | "del" => BareKey::Delete,
            "insert" | "ins" => BareKey::Insert,
            "home" => BareKey::Home,
            "end" => BareKey::End,
            "pageup" | "pgup" => BareKey::PageUp,
            "pagedown" | "pgdn" => BareKey::PageDown,
            "up" => BareKey::Up,
            "down" => BareKey::Down,
            "left" => BareKey::Left,
            "right" => BareKey::Right,
            s if s.starts_with("f") && s.len() > 1 => {
                // F1-F12
                let num: u8 = s[1..]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid function key: {}", s))?;
                BareKey::F(num)
            }
            s if s.len() == 1 => BareKey::Char(s.chars().next().unwrap()),
            _ => anyhow::bail!("Unknown key: {}", key_str),
        };

        Ok((bare_key, modifiers))
    }
}

impl CommandExecutor {
    pub(super) fn write_to_tab(
        context: &mut CliContext,
        text: String,
        add_newline: bool,
    ) -> anyhow::Result<()> {
        let mgr = context
            .manager_mut()
            .with_context(|| "Not attached to any session")?;

        let processed_text = process_escape_sequences(&text);

        let content = if add_newline {
            format!("{}\n", processed_text)
        } else {
            processed_text
        };

        mgr.write_to_pane(content)?;

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

        for cmd in &commands {
            let processed = process_escape_sequences(cmd);
            mgr.write_to_pane(format!("{}\n", processed))?;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

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

        let key_name = format_key_name(&key, &modifiers);

        mgr.send_key(key, modifiers)?;

        println!("{}", format!("✓ Sent {} to current tab", key_name).green());

        Ok(())
    }
}
