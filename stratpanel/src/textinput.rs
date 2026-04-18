const MOD_SHIFT: u32 = 1 << 0;
const MOD_CAPS: u32 = 1 << 1;
const MOD_CTRL: u32 = 1 << 2;

const MAX_INPUT_LEN: usize = 256;
const VISIBLE_CHARS: usize = 30;

pub struct TextInput {
    buffer: Vec<char>,
    cursor: usize,
    scroll: usize,
    shift: bool,
    caps: bool,
    ctrl: bool,
}

impl TextInput {
    pub fn new() -> Self {
        TextInput {
            buffer: Vec::new(),
            cursor: 0,
            scroll: 0,
            shift: false,
            caps: false,
            ctrl: false,
        }
    }

    pub fn handle_modifiers(&mut self, mods_depressed: u32) {
        self.shift = (mods_depressed & MOD_SHIFT) != 0;
        self.caps = (mods_depressed & MOD_CAPS) != 0;
        self.ctrl = (mods_depressed & MOD_CTRL) != 0;
    }

    pub fn handle_key(&mut self, evdev_key: u32) {
        // evdev keycodes: https://www.kernel.org/doc/html/latest/input/event-codes.html
        // Keycodes are offset by 8 from the kernel KEY_* defines
        match evdev_key {
            // Backspace (KEY_BACKSPACE = 14, evdev = 14)
            14 => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.buffer.remove(self.cursor);
                    self.adjust_scroll();
                }
            }
            // Delete (KEY_DELETE = 111, evdev = 111)
            111 => {
                if self.cursor < self.buffer.len() {
                    self.buffer.remove(self.cursor);
                    self.adjust_scroll();
                }
            }
            // Left arrow (KEY_LEFT = 105, evdev = 105)
            105 => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.adjust_scroll();
                }
            }
            // Right arrow (KEY_RIGHT = 106, evdev = 106)
            106 => {
                if self.cursor < self.buffer.len() {
                    self.cursor += 1;
                    self.adjust_scroll();
                }
            }
            // Home (KEY_HOME = 102, evdev = 102)
            102 => {
                self.cursor = 0;
                self.scroll = 0;
            }
            // End (KEY_END = 107, evdev = 107)
            107 => {
                self.cursor = self.buffer.len();
                self.adjust_scroll();
            }
            // Enter (KEY_ENTER = 28, evdev = 28) — submit (no-op for now)
            28 => {}
            // Escape (KEY_ESC = 1, evdev = 1) — clear
            1 => {
                self.buffer.clear();
                self.cursor = 0;
                self.scroll = 0;
            }
            // Printable keys
            key => {
                let ch = self.map_key(key);
                if let Some(c) = ch {
                    if self.buffer.len() < MAX_INPUT_LEN {
                        self.buffer.insert(self.cursor, c);
                        self.cursor += 1;
                        self.adjust_scroll();
                    }
                }
            }
        }
    }

    pub fn click_at(&mut self, pixel_offset: i32) {
        if pixel_offset < 0 {
            self.cursor = 0;
        } else {
            let char_idx = (pixel_offset / 6) as usize;
            self.cursor = char_idx.min(self.buffer.len());
        }
        self.adjust_scroll();
    }

    pub fn display_text(&self) -> String {
        let end = (self.scroll + VISIBLE_CHARS).min(self.buffer.len());
        self.buffer[self.scroll..end].iter().collect()
    }

    pub fn cursor_pixel_offset(&self) -> i32 {
        (self.cursor - self.scroll) as i32 * 6
    }

    fn adjust_scroll(&mut self) {
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + VISIBLE_CHARS {
            self.scroll = self.cursor - VISIBLE_CHARS + 1;
        }
    }

    fn map_key(&self, evdev_key: u32) -> Option<char> {
        let upper = self.shift != self.caps;
        match evdev_key {
            // KEY_Q..KEY_P (rows 1-4 of US QWERTY)
            16 => Some(if upper { 'Q' } else { 'q' }),
            17 => Some(if upper { 'W' } else { 'w' }),
            18 => Some(if upper { 'E' } else { 'e' }),
            19 => Some(if upper { 'R' } else { 'r' }),
            20 => Some(if upper { 'T' } else { 't' }),
            21 => Some(if upper { 'Y' } else { 'y' }),
            22 => Some(if upper { 'U' } else { 'u' }),
            23 => Some(if upper { 'I' } else { 'i' }),
            24 => Some(if upper { 'O' } else { 'o' }),
            25 => Some(if upper { 'P' } else { 'p' }),
            // KEY_A..KEY_L
            30 => Some(if upper { 'A' } else { 'a' }),
            31 => Some(if upper { 'S' } else { 's' }),
            32 => Some(if upper { 'D' } else { 'd' }),
            33 => Some(if upper { 'F' } else { 'f' }),
            34 => Some(if upper { 'G' } else { 'g' }),
            35 => Some(if upper { 'H' } else { 'h' }),
            36 => Some(if upper { 'J' } else { 'j' }),
            37 => Some(if upper { 'K' } else { 'k' }),
            38 => Some(if upper { 'L' } else { 'l' }),
            // KEY_Z..KEY_M
            44 => Some(if upper { 'Z' } else { 'z' }),
            45 => Some(if upper { 'X' } else { 'x' }),
            46 => Some(if upper { 'C' } else { 'c' }),
            47 => Some(if upper { 'V' } else { 'v' }),
            48 => Some(if upper { 'B' } else { 'b' }),
            49 => Some(if upper { 'N' } else { 'n' }),
            50 => Some(if upper { 'M' } else { 'm' }),
            // Number row: KEY_1..KEY_0 (evdev 2..11)
            2 => Some(if upper { '!' } else { '1' }),
            3 => Some(if upper { '@' } else { '2' }),
            4 => Some(if upper { '#' } else { '3' }),
            5 => Some(if upper { '$' } else { '4' }),
            6 => Some(if upper { '%' } else { '5' }),
            7 => Some(if upper { '^' } else { '6' }),
            8 => Some(if upper { '&' } else { '7' }),
            9 => Some(if upper { '*' } else { '8' }),
            10 => Some(if upper { '(' } else { '9' }),
            11 => Some(if upper { ')' } else { '0' }),
            // Minus / underscore (KEY_MINUS = 12)
            12 => Some(if upper { '_' } else { '-' }),
            // Equal / plus (KEY_EQUAL = 13)
            13 => Some(if upper { '+' } else { '=' }),
            // Left/right bracket
            26 => Some(if upper { '{' } else { '[' }),
            27 => Some(if upper { '}' } else { ']' }),
            // Semicolon / colon (KEY_SEMICOLON = 39)
            39 => Some(if upper { ':' } else { ';' }),
            // Quote / double-quote (KEY_APOSTROPHE = 40)
            40 => Some(if upper { '"' } else { '\'' }),
            // Backtick / tilde (KEY_GRAVE = 41)
            41 => Some(if upper { '~' } else { '`' }),
            // Backslash / pipe (KEY_BACKSLASH = 43)
            43 => Some(if upper { '|' } else { '\\' }),
            // Comma / less-than (KEY_COMMA = 51)
            51 => Some(if upper { '<' } else { ',' }),
            // Period / greater-than (KEY_DOT = 52)
            52 => Some(if upper { '>' } else { '.' }),
            // Slash / question-mark (KEY_SLASH = 53)
            53 => Some(if upper { '?' } else { '/' }),
            // Space (KEY_SPACE = 57)
            57 => Some(' '),
            // Tab (KEY_TAB = 15)
            15 => Some('\t'),
            _ => None,
        }
    }
}
