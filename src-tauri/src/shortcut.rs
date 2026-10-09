use tauri_plugin_global_shortcut::Shortcut;

use crate::model::{Binding, ShortcutCandidate, ShortcutCheck, Sound};

impl Sound {
    pub fn shortcut_label(&self) -> String {
        shortcut_label(&self.binding)
    }
}

const RESERVED: &[&str] = &[
    // Clipboard, editing, find and other commands nearly every app handles.
    "Ctrl+A", "Ctrl+C", "Ctrl+F", "Ctrl+H", "Ctrl+N", "Ctrl+O", "Ctrl+P", "Ctrl+S",
    "Ctrl+V", "Ctrl+W", "Ctrl+X", "Ctrl+Y", "Ctrl+Z",
    // Developer tools.
    "F5", "F11", "F12", "Ctrl+F5", "Ctrl+Shift+C", "Ctrl+Shift+I", "Ctrl+Shift+J",
    "Ctrl+Shift+P",
    // Windows and system combinations.
    "Alt+Esc", "Alt+F4", "Alt+Tab", "Ctrl+Esc", "Win+D", "Win+E", "Win+L", "Win+R", "Win+Tab",
    "PrtScn", "Pause", "Ctrl+Alt+Del", "Ctrl+Alt+Backspace",
];

fn strip_digits<'a>(code: &'a str, prefix: &str, max_len: usize) -> Option<&'a str> {
    let rest = code.strip_prefix(prefix)?;
    if rest.is_empty() || rest.len() > max_len || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(rest)
}

fn strip_single<'a>(code: &'a str, prefix: &str, accept: fn(char) -> bool) -> Option<&'a str> {
    let rest = code.strip_prefix(prefix)?;
    let mut chars = rest.chars();
    let first = chars.next()?;
    if chars.next().is_some() || !accept(first) {
        return None;
    }
    Some(rest)
}

/// Human readable name for a browser `KeyboardEvent.code` value, or `None`
/// when the key is not offered as a global shortcut.
pub fn key_label(code: &str) -> Option<String> {
    if let Some(letter) = strip_single(code, "Key", |c| c.is_ascii_uppercase()) {
        return Some(letter.to_string());
    }
    if let Some(digit) = strip_single(code, "Digit", |c| c.is_ascii_digit()) {
        return Some(digit.to_string());
    }
    if let Some(digit) = strip_single(code, "Numpad", |c| c.is_ascii_digit()) {
        return Some(format!("Num {digit}"));
    }
    if let Some(number) = strip_digits(code, "F", 2) {
        return Some(format!("F{number}"));
    }

    let label = match code {
        "Space" => "Space",
        "Enter" | "NumpadEnter" => "Enter",
        "Tab" => "Tab",
        "Escape" => "Esc",
        "Backspace" | "NumpadBackspace" => "Backspace",
        "Delete" => "Del",
        "Insert" => "Ins",
        "Home" => "Home",
        "End" => "End",
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        "PrintScreen" => "PrtScn",
        "ScrollLock" => "ScrLk",
        "Pause" => "Pause",
        "CapsLock" => "CapsLk",
        "NumLock" => "NumLk",
        "ContextMenu" => "Menu",
        "Semicolon" => ";",
        "Equal" => "=",
        "Comma" => ",",
        "Minus" => "-",
        "Period" => ".",
        "Slash" => "/",
        "Backslash" => "\\",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Quote" => "'",
        "Backquote" => "`",
        "NumpadAdd" => "Num +",
        "NumpadSubtract" => "Num -",
        "NumpadMultiply" => "Num *",
        "NumpadDivide" => "Num /",
        "NumpadDecimal" => "Num .",
        _ => return None,
    };
    Some(label.to_string())
}

/// Maps a Windows virtual-key code to the same `code` vocabulary the browser
/// recorder produces, so hook-observed presses and recorded presses compare
/// equal.
pub fn code_from_vk(vk: u32) -> Option<String> {
    let code = match vk {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x13 => "Pause",
        0x14 => "CapsLock",
        0x1B => "Escape",
        0x20 => "Space",
        0x21 => "PageUp",
        0x22 => "PageDown",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2C => "PrintScreen",
        0x2D => "Insert",
        0x2E => "Delete",
        0x5D => "ContextMenu",
        0x90 => "NumLock",
        0x91 => "ScrollLock",
        0xBA => "Semicolon",
        0xBB => "Equal",
        0xBC => "Comma",
        0xBD => "Minus",
        0xBE => "Period",
        0xBF => "Slash",
        0xC0 => "Backquote",
        0xDB => "BracketLeft",
        0xDC => "Backslash",
        0xDD => "BracketRight",
        0xDE => "Quote",
        0x6A => "NumpadMultiply",
        0x6B => "NumpadAdd",
        0x6D => "NumpadSubtract",
        0x6E => "NumpadDecimal",
        0x6F => "NumpadDivide",
        0x30..=0x39 => return Some(format!("Digit{}", char::from_u32(vk)?)),
        0x41..=0x5A => return Some(format!("Key{}", char::from_u32(vk)?)),
        0x60..=0x69 => return Some(format!("Numpad{}", char::from_u32(vk)?)),
        0x70..=0x87 => return Some(format!("F{}", vk - 0x70 + 1)),
        _ => return None,
    };
    Some(code.to_string())
}

pub fn shortcut_label(binding: &Binding) -> String {
    let mut parts: Vec<String> = Vec::new();
    if binding.ctrl {
        parts.push("Ctrl".to_string());
    }
    if binding.alt {
        parts.push("Alt".to_string());
    }
    if binding.shift {
        parts.push("Shift".to_string());
    }
    if binding.meta {
        parts.push("Win".to_string());
    }
    if let Some(label) = key_label(&binding.code) {
        parts.push(label);
    }
    parts.join("+")
}

/// Accelerator string understood by the global shortcut plugin. Modifier order
/// is fixed so the same binding always produces the same string.
pub fn accelerator(binding: &Binding) -> Result<String, String> {
    if key_label(&binding.code).is_none() {
        return Err(format!("{} is not a key this app can bind.", describe_key(binding)));
    }
    let mut parts = Vec::new();
    if binding.ctrl {
        parts.push("Ctrl");
    }
    if binding.alt {
        parts.push("Alt");
    }
    if binding.shift {
        parts.push("Shift");
    }
    if binding.meta {
        parts.push("Super");
    }
    parts.push(binding.code.as_str());
    Ok(parts.join("+"))
}

pub fn parse(shortcut: &str) -> Result<Shortcut, String> {
    shortcut
        .parse::<Shortcut>()
        .map_err(|_| format!("{shortcut} is not a shortcut this app understands."))
}

fn describe_key(binding: &Binding) -> String {
    key_label(&binding.code).unwrap_or_else(|| binding.code.clone())
}

fn reserved(binding: &Binding) -> bool {
    let label = shortcut_label(binding);
    RESERVED.contains(&label.as_str())
}

fn has_modifier(binding: &Binding) -> bool {
    binding.ctrl || binding.alt || binding.shift || binding.meta
}

fn requires_modifier(binding: &Binding) -> bool {
    strip_single(&binding.code, "Key", |c| c.is_ascii_uppercase()).is_some()
        || strip_single(&binding.code, "Digit", |c| c.is_ascii_digit()).is_some()
        || matches!(
            binding.code.as_str(),
            "Space" | "Period" | "Comma" | "Slash" | "Semicolon" | "Quote" | "Minus" | "Equal"
                | "BracketLeft" | "BracketRight" | "Backslash" | "Backquote"
        )
}

pub fn check(binding: &Binding, taken_by_other: Option<&str>) -> ShortcutCheck {
    if key_label(&binding.code).is_none() {
        return ShortcutCheck::rejected(format!(
            "{} is not a key this app can bind. Try a letter, number, function key or arrow key.",
            describe_key(binding)
        ));
    }

    let shortcut = shortcut_label(binding);

    let error = if reserved(binding) {
        Some(format!(
            "{shortcut} is reserved by Windows or common apps, so binding it would break normal use."
        ))
    } else if let Some(owner) = taken_by_other {
        Some(format!("{shortcut} is already assigned to \"{owner}\"."))
    } else {
        None
    };

    if let Some(error) = error {
        return ShortcutCheck {
            shortcut,
            ok: false,
            warning: None,
            error: Some(error),
        };
    }

    let warning = if !has_modifier(binding) && requires_modifier(binding) {
        Some(format!(
            "{shortcut} has no modifier key, so it will swallow that key in every app while the soundboard runs. Add a modifier such as Ctrl or Alt unless you meant it."
        ))
    } else {
        None
    };

    ShortcutCheck {
        shortcut,
        ok: true,
        warning,
        error: None,
    }
}

pub fn from_candidate(candidate: ShortcutCandidate) -> Binding {
    Binding::from(candidate)
}

/// Modifier state as read from the keyboard, so hook-observed presses and
/// browser-recorded presses agree on what "Ctrl+G" means.
#[derive(Debug, Default, Clone, Copy)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// Builds a binding from a virtual-key code plus observed modifiers. Returns
/// `None` for keys the app will not bind.
pub fn binding_from_vk(vk: u32, mods: &Modifiers) -> Option<Binding> {
    let code = code_from_vk(vk)?;
    Some(Binding {
        ctrl: mods.ctrl,
        alt: mods.alt,
        shift: mods.shift,
        meta: mods.meta,
        code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(code: &str, ctrl: bool, alt: bool, shift: bool, meta: bool) -> Binding {
        Binding {
            ctrl,
            alt,
            shift,
            meta,
            code: code.to_string(),
        }
    }

    #[test]
    fn labels_letters_digits_and_functions() {
        assert_eq!(key_label("KeyG").as_deref(), Some("G"));
        assert_eq!(key_label("Digit7").as_deref(), Some("7"));
        assert_eq!(key_label("F12").as_deref(), Some("F12"));
        assert_eq!(key_label("Numpad3").as_deref(), Some("Num 3"));
        assert_eq!(key_label("Semicolon").as_deref(), Some(";"));
        assert_eq!(key_label("Fn").as_deref(), None);
        assert_eq!(key_label("ShiftLeft").as_deref(), None);
        assert_eq!(key_label("MetaRight").as_deref(), None);
    }

    #[test]
    fn label_orders_modifiers_deterministically() {
        let binding = binding("KeyG", true, true, true, true);
        assert_eq!(shortcut_label(&binding), "Ctrl+Alt+Shift+Win+G");
    }

    #[test]
    fn rejects_reserved_and_duplicate_bindings() {
        let clipboard = binding("KeyC", true, false, false, false);
        assert!(!check(&clipboard, None).ok);
        assert!(!check(&clipboard, None).error.unwrap().is_empty());

        let free = binding("KeyG", true, false, true, false);
        let duplicate = check(&free, Some("Air Horn"));
        assert!(!duplicate.ok);
        assert!(duplicate.error.unwrap().contains("Air Horn"));
        assert!(check(&free, None).ok);
    }

    #[test]
    fn warns_but_allows_bare_typing_keys() {
        let bare = check(&binding("KeyG", false, false, false, false), None);
        assert!(bare.ok);
        assert!(bare.warning.is_some());

        let bare_function = check(&binding("F6", false, false, false, false), None);
        assert!(bare_function.ok);
        assert!(bare_function.warning.is_none());
    }

    #[test]
    fn blocks_clipboard_and_editing_but_allows_other_ctrl_letters() {
        for code in ["KeyA", "KeyC", "KeyF", "KeyH", "KeyN", "KeyO", "KeyP", "KeyS", "KeyV",
                     "KeyW", "KeyX", "KeyY", "KeyZ"] {
            let binding = binding(code, true, false, false, false);
            assert!(!check(&binding, None).ok, "{code} should stay reserved");
        }

        for code in ["KeyB", "KeyD", "KeyE", "KeyG", "KeyI", "KeyJ", "KeyK", "KeyL", "KeyM",
                     "KeyQ", "KeyR", "KeyT", "KeyU"] {
            let binding = binding(code, true, false, false, false);
            assert!(check(&binding, None).ok, "{} should be bindable", shortcut_label(&binding));
        }
    }

    #[test]
    fn keeps_windows_and_developer_tools_reserved() {
        for (code, ctrl, alt, meta) in [
            ("F5", false, false, false), ("F11", false, false, false), ("F12", false, false, false),
            ("Tab", false, true, false), ("F4", false, true, false), ("Escape", true, false, false),
            ("KeyL", false, false, true), ("KeyD", false, false, true), ("KeyR", false, false, true),
        ] {
            let binding = binding(code, ctrl, alt, false, meta);
            assert!(!check(&binding, None).ok, "{code} must stay reserved");
        }
    }

    #[test]
    fn accelerators_parse_with_the_plugin() {
        for binding in [
            binding("KeyG", true, false, true, false),
            binding("F6", false, false, false, false),
            binding("KeyM", true, true, false, true),
            binding("Space", false, false, false, false),
        ] {
            let accelerator = accelerator(&binding).unwrap();
            assert!(parse(&accelerator).is_ok(), "{accelerator} should parse");
        }
    }
}