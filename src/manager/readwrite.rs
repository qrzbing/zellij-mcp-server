use std::{collections::BTreeSet, fs, path::PathBuf};

use anyhow::Context;
use chrono::Local;
use tracing::debug;
use zellij_utils::{
    cli::CliAction,
    data::{BareKey, KeyModifier},
};

use super::{
    ZellijSessionManager,
    utils::{format_key_name, parse_key_string, process_escape_sequences},
};

#[derive(Debug, Copy, Clone)]
pub enum DumpRange {
    Viewport,                           // viewport
    Last(usize),                        // --lines N
    Range { begin: usize, end: usize }, // --begin N --end N（1-indexed）
}

fn key_to_bytes(key: &BareKey, modifiers: &BTreeSet<KeyModifier>) -> anyhow::Result<Vec<u8>> {
    if modifiers.contains(&KeyModifier::Ctrl) {
        if let BareKey::Char(c) = key {
            let byte = (c.to_ascii_uppercase() as u8) & 0x1F;
            return Ok(vec![byte]);
        }
    }

    match key {
        BareKey::Enter => Ok(vec![0x0D]), // LF
        BareKey::Tab => Ok(vec![0x09]),
        BareKey::Backspace => Ok(vec![0x08]),
        BareKey::Esc => Ok(vec![0x1B]),
        BareKey::Delete => Ok(vec![0x1B, 0x5B, 0x33, 0x7E]), // ESC[3~
        BareKey::Insert => Ok(vec![0x1B, 0x5B, 0x32, 0x7E]), // ESC[2~
        BareKey::Home => Ok(vec![0x1B, 0x5B, 0x48]),         // ESC[H
        BareKey::End => Ok(vec![0x1B, 0x5B, 0x46]),          // ESC[F
        BareKey::PageUp => Ok(vec![0x1B, 0x5B, 0x35, 0x7E]), // ESC[5~
        BareKey::PageDown => Ok(vec![0x1B, 0x5B, 0x36, 0x7E]), // ESC[6~
        BareKey::Up => Ok(vec![0x1B, 0x5B, 0x41]),           // ESC[A
        BareKey::Down => Ok(vec![0x1B, 0x5B, 0x42]),         // ESC[B
        BareKey::Right => Ok(vec![0x1B, 0x5B, 0x43]),        // ESC[C
        BareKey::Left => Ok(vec![0x1B, 0x5B, 0x44]),         // ESC[D
        BareKey::F(n) => {
            // F1-F12
            match n {
                1 => Ok(vec![0x1B, 0x4F, 0x50]), // ESC OP
                2 => Ok(vec![0x1B, 0x4F, 0x51]), // ESC OQ
                3 => Ok(vec![0x1B, 0x4F, 0x52]), // ESC OR
                4 => Ok(vec![0x1B, 0x4F, 0x53]), // ESC OS
                5..=12 => {
                    let code = match n {
                        5 => 0x31,
                        6 => 0x37,
                        7 => 0x38,
                        8 => 0x39,
                        9 => 0x30,
                        10 => 0x31,
                        11 => 0x33,
                        12 => 0x34,
                        _ => unreachable!(),
                    };
                    Ok(vec![0x1B, 0x5B, 0x31, code, 0x7E])
                }
                _ => anyhow::bail!("Unsupported function key: F{}", n),
            }
        }
        BareKey::Char(c) => Ok(c.to_string().into_bytes()),
        _ => anyhow::bail!("Unsupported key: {:?}", key),
    }
}

impl ZellijSessionManager {
    // ============ High-level APIs ============

    /// Write text to the current pane (automatically processes escape sequences)
    ///
    /// # Arguments
    /// * `text` - Text to write (supports escape sequences like \n, \t, \e)
    /// * `add_newline` - Whether to add a newline at the end
    ///
    /// # Examples
    /// ```
    /// mgr.write_text("echo hello", true)?;  // Execute command
    /// mgr.write_text("hello", false)?;       // Write text only
    /// ```
    pub fn write_text(&mut self, text: &str, add_newline: bool) -> anyhow::Result<String> {
        let processed_text = process_escape_sequences(text);
        let content = if add_newline {
            format!("{}\n", processed_text)
        } else {
            processed_text
        };

        self.write_to_pane(content)?;

        std::thread::sleep(std::time::Duration::from_millis(100));

        let result = self.dump_for_write();
        let content = match result {
            Ok(content) => content,
            Err(_) => "[-] Failed to dump screen".to_string(),
        };

        Ok(content)
    }

    /// Write multiple commands to the pane (each automatically appends newline)
    ///
    /// # Arguments
    /// * `commands` - List of commands to execute
    ///
    /// # Examples
    /// ```
    /// mgr.write_multiple(&["cd /tmp".to_string(), "ls -la".to_string()])?;
    /// ```
    pub fn write_multiple(&mut self, commands: &[String]) -> anyhow::Result<String> {
        for cmd in commands {
            let processed = process_escape_sequences(cmd);
            self.write_to_pane(format!("{}\n", processed))?;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        std::thread::sleep(std::time::Duration::from_millis(100));

        let result = self.dump_for_write();
        let content = match result {
            Ok(content) => content,
            Err(_) => "[-] Failed to dump screen".to_string(),
        };

        Ok(content)
    }

    /// Send a key by parsing a key string
    ///
    /// # Arguments
    /// * `key_str` - Key string (e.g., "ctrl+c", "enter", "f1")
    ///
    /// # Examples
    /// ```
    /// mgr.send_key_string("ctrl+c")?;
    /// mgr.send_key_string("enter")?;
    /// ```
    pub fn send_key_string(&self, key_str: &str) -> anyhow::Result<String> {
        let (bare_key, modifiers) = parse_key_string(key_str)?;
        let key_name = format_key_name(&bare_key, &modifiers);

        self.send_key(bare_key, modifiers)?;

        debug!("Sent key to pane: {}", key_name);

        Ok(key_name)
    }

    // ============ Low-level APIs ============

    /// Write text directly to the pane (no escape sequence processing)
    pub fn write_to_pane(&self, text: String) -> anyhow::Result<()> {
        self.send_action(
            CliAction::WriteChars {
                chars: text.clone(),
            },
            None,
        )?;

        debug!("Wrote to pane: {:?}", text);

        Ok(())
    }

    pub fn send_bytes(&self, bytes: Vec<u8>) -> anyhow::Result<()> {
        self.send_action(
            CliAction::Write {
                bytes: bytes.clone(),
            },
            None,
        )?;
        debug!("Sent bytes to pane: {:?}", bytes);
        Ok(())
    }

    pub fn send_key(&self, key: BareKey, modifiers: BTreeSet<KeyModifier>) -> anyhow::Result<()> {
        let bytes = key_to_bytes(&key, &modifiers)?;

        self.send_bytes(bytes)?;

        Ok(())
    }

    fn dump_screen_lines(&self) -> anyhow::Result<(Vec<String>, Option<PathBuf>)> {
        let err_context = || "Failed to dump screen";

        let (dump_file_path, do_remove) = match self.dump_screen_dir {
            Some(ref path) => {
                let current_time = Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();
                (
                    PathBuf::from(path).join(format!("zellij-dump-{}.txt", current_time)),
                    false,
                )
            }
            None => (
                std::env::temp_dir().join(format!("zellij-dump-{}.txt", std::process::id())),
                true,
            ),
        };

        self.send_action(
            CliAction::DumpScreen {
                path: dump_file_path.clone(),
                full: true,
            },
            None,
        )
        .with_context(err_context)?;

        std::thread::sleep(std::time::Duration::from_millis(100));

        let content = fs::read_to_string(&dump_file_path)
            .with_context(|| format!("Failed to read temp file: {:?}", dump_file_path))?;

        let all_lines: Vec<&str> = content.lines().collect();
        let first_valid = all_lines.iter().position(|line| !line.trim().is_empty());
        let last_valid = all_lines.iter().rposition(|line| !line.trim().is_empty());

        let cleaned_lines = match (first_valid, last_valid) {
            (Some(start), Some(end)) => &all_lines[start..=end],
            _ => &[] as &[&str],
        };

        let cleaned_vec: Vec<String> = cleaned_lines.iter().map(|&line| line.to_string()).collect();

        if do_remove {
            let _ = fs::remove_file(&dump_file_path);
            Ok((cleaned_vec, None))
        } else {
            Ok((cleaned_vec, Some(dump_file_path)))
        }
    }

    /// Dump the screen after writing command.
    /// Lines are less than 20.
    fn dump_for_write(&mut self) -> anyhow::Result<String> {
        let (lines, _) = self.dump_screen_lines()?;

        let mut new_start = 0;

        if let Some(ref old_lines) = self.last_dump_message {
            let match_end = old_lines.len().saturating_sub(1);

            let common = (0..std::cmp::min(match_end, lines.len()))
                .take_while(|&i| old_lines[i] == lines[i])
                .count();

            if common > 0 {
                new_start = common.saturating_sub(1);
            } else {
                let max_possible_overlap = std::cmp::min(match_end, lines.len());
                for i in (1..=max_possible_overlap).rev() {
                    if old_lines[match_end - i..match_end] == lines[..i] {
                        new_start = i.saturating_sub(1);
                        break;
                    }
                }
            }
        }

        let result_lines = &lines[new_start..];
        let start_idx = result_lines.len().saturating_sub(20);
        let final_lines = &result_lines[start_idx..];
        let res_string = final_lines.join("\n");

        self.last_dump_message = Some(lines);

        Ok(res_string)
    }

    pub fn dump_screen(&self, range: &DumpRange) -> anyhow::Result<(String, Option<PathBuf>)> {
        let (cleaned_vec, dump_file) = self.dump_screen_lines()?;

        let result = match range {
            DumpRange::Viewport => cleaned_vec.join("\n"),
            &DumpRange::Last(n) => cleaned_vec
                .into_iter()
                .rev()
                .take(n)
                .rev()
                .collect::<Vec<_>>()
                .join("\n"),
            &DumpRange::Range { begin, end } => cleaned_vec
                .into_iter()
                .skip(begin.saturating_sub(1))
                .take(end.saturating_sub(begin) + 1)
                .collect::<Vec<_>>()
                .join("\n"),
        };

        Ok((result, dump_file))
    }
}
