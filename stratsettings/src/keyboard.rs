//! Hotkey strings (`super+space`, `ctrl+shift+q`) → wlroots modifier mask + XKB keysym,
//! and export for `stratvm` (`/config/strat/stratvm-keybinds`).

use std::fs;
use std::path::Path;

use crate::KeyboardSettings;

/// wlroots `enum wlr_keyboard_modifier` (see `wlr/types/wlr_keyboard.h`).
pub const WLR_MOD_SHIFT: u32 = 1 << 0;
pub const WLR_MOD_CTRL: u32 = 1 << 2;
pub const WLR_MOD_ALT: u32 = 1 << 3;
pub const WLR_MOD_LOGO: u32 = 1 << 6;

/// XKB keysyms (subset; matches `xkbcommon-keysyms.h` for these names).
fn keysym_from_token(tok: &str) -> Option<u32> {
    let t = tok.trim().to_ascii_lowercase();
    if t.len() == 1 {
        let c = t.chars().next()?;
        return match c {
            'a'..='z' => Some(c as u32),
            'A'..='Z' => Some(c.to_ascii_lowercase() as u32),
            '0'..='9' => Some(c as u32),
            _ => None,
        };
    }
    match t.as_str() {
        "space" => Some(0x20),
        "return" | "enter" => Some(0xff0d),
        "tab" => Some(0xff09),
        "escape" | "esc" => Some(0xff1b),
        "backspace" => Some(0xff08),
        "period" | "." => Some(0x2e),
        "comma" => Some(0x2c),
        "slash" => Some(0x2f),
        "grave" | "`" => Some(0x60),
        "minus" => Some(0x2d),
        "equal" => Some(0x3d),
        "left" => Some(0xff51),
        "up" => Some(0xff52),
        "right" => Some(0xff53),
        "down" => Some(0xff54),
        "home" => Some(0xff50),
        "end" => Some(0xff57),
        "pageup" => Some(0xff55),
        "pagedown" => Some(0xff56),
        _ => None,
    }
}

/// Parse `super+period`-style specs into `(wlr_modifiers, xkb_keysym)`.
pub fn parse_hotkey(spec: &str) -> Option<(u32, u32)> {
    let s = spec.trim();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s
        .split('+')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    let key_token = *parts.last()?;
    let mod_parts = &parts[..parts.len().saturating_sub(1)];
    let mut mods = 0u32;
    for p in mod_parts {
        match p.to_ascii_lowercase().as_str() {
            "super" | "win" | "meta" | "logo" => mods |= WLR_MOD_LOGO,
            "ctrl" | "control" => mods |= WLR_MOD_CTRL,
            "alt" => mods |= WLR_MOD_ALT,
            "shift" => mods |= WLR_MOD_SHIFT,
            _ => return None,
        }
    }
    let sym = keysym_from_token(key_token)?;
    Some((mods, sym))
}

/// Write `stratvm` keybind file: each line `name decimal_mods decimal_keysym`.
pub fn write_stratvm_keybind_file(root: &Path, k: &KeyboardSettings) -> Result<(), String> {
    let spot = parse_hotkey(&k.spotlite).ok_or_else(|| {
        format!(
            "invalid keyboard.spotlite hotkey: {:?}",
            k.spotlite
        )
    })?;
    let cyc = parse_hotkey(&k.cycle_layout).ok_or_else(|| {
        format!(
            "invalid keyboard.cycle_layout hotkey: {:?}",
            k.cycle_layout
        )
    })?;
    let path = root.join("stratvm-keybinds");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = format!(
        "# Written by stratsettings — Spotlite + layout cycle (stratvm reads at startup / reload_keybinds IPC)\nspotlite {} {}\ncycle_layout {} {}\n",
        spot.0, spot.1, cyc.0, cyc.1
    );
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_super_space() {
        let (m, s) = parse_hotkey("super+space").unwrap();
        assert_eq!(m, WLR_MOD_LOGO);
        assert_eq!(s, 0x20);
    }

    #[test]
    fn parses_super_period() {
        let (m, s) = parse_hotkey("super+period").unwrap();
        assert_eq!(m, WLR_MOD_LOGO);
        assert_eq!(s, 0x2e);
    }
}
