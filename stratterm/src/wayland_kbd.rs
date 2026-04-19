//! XKB state for `wl_keyboard`: resolves Linux evdev key codes + layout to UTF-8 / keysyms.

use xkbcommon::xkb::{self, KeyDirection, Keycode};

pub struct WaylandKeyState {
    keymap: Option<xkb::Keymap>,
    state: Option<xkb::State>,
}

impl WaylandKeyState {
    pub fn new() -> Self {
        Self {
            keymap: None,
            state: None,
        }
    }

    /// Handle `wl_keyboard.keymap` payload (already read from the map fd by stratlayer).
    pub fn apply_keymap(&mut self, format: u32, data: &[u8]) -> Result<(), String> {
        const FMT_NONE: u32 = 0;
        const FMT_XKB_V1: u32 = 1;

        self.state = None;
        self.keymap = None;

        if format == FMT_NONE || data.is_empty() {
            return Ok(());
        }
        if format != FMT_XKB_V1 {
            return Err(format!("unsupported wl_keyboard keymap format {format}"));
        }

        let text = String::from_utf8_lossy(data).into_owned();
        let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_string(
            &ctx,
            text,
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or_else(|| "invalid XKB keymap".to_string())?;
        let state = xkb::State::new(&keymap);
        self.keymap = Some(keymap);
        self.state = Some(state);
        Ok(())
    }

    pub fn update_modifiers(&mut self, depressed: u32, latched: u32, locked: u32, group: u32) {
        let Some(st) = self.state.as_mut() else {
            return;
        };
        let _ = st.update_mask(depressed, latched, locked, group, 0, 0);
    }

    /// `key` is the `wl_keyboard.key` value (Linux evdev code). Call for press and release.
    /// On press, returns `(true, utf8, keysym_raw)`; on release `(false, "", 0)`.
    ///
    /// On press, resolve keysyms/UTF-8 before `State::update_key` (Down); libxkbcommon documents
    /// that order so this event’s translation is not altered by the update (dead keys, compose, latches).
    pub fn on_key(&mut self, key: u32, pressed: bool) -> (bool, String, u32) {
        let Some(st) = self.state.as_mut() else {
            return (false, String::new(), 0);
        };
        let kc = Keycode::new(key + 8);
        if pressed {
            let sym = st.key_get_one_sym(kc);
            let utf8 = st.key_get_utf8(kc);
            st.update_key(kc, KeyDirection::Down);
            (true, utf8, sym.raw())
        } else {
            st.update_key(kc, KeyDirection::Up);
            (false, String::new(), 0)
        }
    }
}
