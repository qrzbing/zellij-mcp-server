use std::{collections::BTreeSet, fs, path::PathBuf};

use anyhow::Context;
use tracing::debug;
use zellij_utils::{
    cli::CliAction,
    data::{BareKey, KeyModifier},
};

use super::ZellijSessionManager;

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

    pub fn dump_screen(&self, path: Option<String>, full: bool) -> anyhow::Result<String> {
        let err_context = || "Failed to dump screen";

        if let Some(file_path) = path {
            let path_buf = PathBuf::from(file_path);

            self.send_action(
                CliAction::DumpScreen {
                    path: path_buf.clone(),
                    full,
                },
                None,
            )
            .with_context(err_context)?;

            debug!("Dumped screen to file: {:?}", path_buf);

            Ok(String::new())
        } else {
            let temp_file =
                std::env::temp_dir().join(format!("zellij-dump-{}.txt", std::process::id()));

            self.send_action(
                CliAction::DumpScreen {
                    path: temp_file.clone(),
                    full,
                },
                None,
            )
            .with_context(err_context)?;

            std::thread::sleep(std::time::Duration::from_millis(100));

            let content = fs::read_to_string(&temp_file)
                .with_context(|| format!("Failed to read temp file: {:?}", temp_file))?;

            let _ = fs::remove_file(&temp_file);

            debug!("Dumped screen to stdout (via temp file)");

            Ok(content)
        }
    }
}
