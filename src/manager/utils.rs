use std::collections::BTreeSet;
use zellij_utils::data::{BareKey, KeyModifier};

/// Process escape sequences in input string
///
/// Supported escape sequences:
/// - \r - Carriage Return (CR, 0x0D)
/// - \n - Line Feed (LF, 0x0A)
/// - \t - Tab (0x09)
/// - \0 - Null (0x00)
/// - \e - ESC (0x1B)
/// - \\ - Backslash
/// - \xHH - Hexadecimal byte
pub fn process_escape_sequences(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'r' => result.push('\r'),
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    '0' => result.push('\0'),
                    'e' => result.push('\x1B'),
                    '\\' => result.push('\\'),
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

/// Parse key string to BareKey and modifiers
///
/// Supported formats:
/// - ctrl+c, ctrl+d, ctrl+z, ctrl+l
/// - ^c, ^d (ctrl shorthand)
/// - alt+x
/// - enter, return, tab, backspace, esc, delete
/// - up, down, left, right
/// - home, end, pageup, pagedown
/// - f1-f12
/// - single character
pub fn parse_key_string(key_str: &str) -> anyhow::Result<(BareKey, BTreeSet<KeyModifier>)> {
    let mut modifiers = BTreeSet::new();
    let key_str = key_str.to_lowercase();

    // Ctrl+Key or ^Key
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

    // Alt+Key
    if key_str.starts_with("alt+") {
        modifiers.insert(KeyModifier::Alt);
        let base = &key_str[4..];
        let bare_key = BareKey::Char(base.chars().next().unwrap());
        return Ok((bare_key, modifiers));
    }

    // Special keys
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

/// Format key name for display
pub fn format_key_name(key: &BareKey, modifiers: &BTreeSet<KeyModifier>) -> String {
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
