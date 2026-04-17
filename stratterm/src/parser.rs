use crate::screen::{ScreenBuffer, Color};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParserState {
    Ground,
    Escape,
    CSI,
    CSIParam,
    OSC,
}

pub struct VtParser {
    state: ParserState,
    params: Vec<u16>,
    current_param: String,
    osc_data: String,
}

impl VtParser {
    pub fn new() -> Self {
        VtParser {
            state: ParserState::Ground,
            params: Vec::new(),
            current_param: String::new(),
            osc_data: String::new(),
        }
    }

    pub fn parse(&mut self, screen: &mut ScreenBuffer, data: &[u8]) {
        for &byte in data {
            self.parse_byte(screen, byte);
        }
    }

    fn parse_byte(&mut self, screen: &mut ScreenBuffer, byte: u8) {
        match self.state {
            ParserState::Ground => {
                match byte {
                    0x1B => {
                        self.state = ParserState::Escape;
                    }
                    0x0A => {
                        // Line feed
                        screen.cursor_row += 1;
                        if screen.cursor_row >= screen.rows {
                            screen.scroll_up(1);
                            screen.cursor_row = screen.rows - 1;
                        }
                    }
                    0x0D => {
                        // Carriage return
                        screen.cursor_col = 0;
                    }
                    0x08 => {
                        // Backspace
                        if screen.cursor_col > 0 {
                            screen.cursor_col -= 1;
                        }
                    }
                    0x09 => {
                        // Tab - move to next multiple of 8
                        let next_tab = ((screen.cursor_col / 8) + 1) * 8;
                        screen.cursor_col = next_tab.min(screen.cols - 1);
                    }
                    0x00..=0x1F | 0x7F => {
                        // Other control characters - ignore for now
                    }
                    _ => {
                        // Printable character
                        let ch = byte as char;
                        screen.put_char(ch);
                    }
                }
            }
            ParserState::Escape => {
                match byte {
                    b'[' => {
                        self.state = ParserState::CSI;
                        self.params.clear();
                        self.current_param.clear();
                    }
                    b']' => {
                        self.state = ParserState::OSC;
                        self.osc_data.clear();
                    }
                    b'M' => {
                        // Reverse index (scroll down)
                        screen.cursor_row = screen.cursor_row.saturating_sub(1);
                        if screen.cursor_row < screen.scroll_top {
                            // Scroll down (move content down)
                            let scroll_start = screen.scroll_top;
                            let scroll_end = screen.scroll_bottom;
                            if scroll_start < scroll_end {
                                for i in (scroll_start + 1)..=scroll_end {
                                    if i < screen.rows && i > 0 {
                                        screen.cells[i - 1] = screen.cells[i].clone();
                                    }
                                }
                                for cell in &mut screen.cells[scroll_end] {
                                    *cell = Default::default();
                                }
                            }
                        }
                        self.state = ParserState::Ground;
                    }
                    b'D' => {
                        // Index (scroll up)
                        screen.cursor_row += 1;
                        if screen.cursor_row >= screen.rows {
                            screen.scroll_up(1);
                            screen.cursor_row = screen.rows - 1;
                        }
                        self.state = ParserState::Ground;
                    }
                    b'E' => {
                        // Next line
                        screen.cursor_row += 1;
                        screen.cursor_col = 0;
                        if screen.cursor_row >= screen.rows {
                            screen.scroll_up(1);
                            screen.cursor_row = screen.rows - 1;
                        }
                        self.state = ParserState::Ground;
                    }
                    b'7' => {
                        // Save cursor - not implemented for MVP
                        self.state = ParserState::Ground;
                    }
                    b'8' => {
                        // Restore cursor - not implemented for MVP
                        self.state = ParserState::Ground;
                    }
                    _ => {
                        // Unknown escape sequence, return to ground
                        self.state = ParserState::Ground;
                    }
                }
            }
            ParserState::CSI => {
                match byte {
                    b'0'..=b'9' | b';' => {
                        if byte == b';' {
                            if !self.current_param.is_empty() {
                                if let Ok(param) = self.current_param.parse::<u16>() {
                                    self.params.push(param);
                                }
                                self.current_param.clear();
                            }
                        } else {
                            self.current_param.push(byte as char);
                        }
                    }
                    b'm' => {
                        // SGR - Select Graphic Rendition
                        if !self.current_param.is_empty() {
                            if let Ok(param) = self.current_param.parse::<u16>() {
                                self.params.push(param);
                            }
                        }
                        self.handle_sgr(screen);
                        self.state = ParserState::Ground;
                    }
                    b'H' => {
                        // Cursor position
                        if !self.current_param.is_empty() {
                            if let Ok(param) = self.current_param.parse::<u16>() {
                                self.params.push(param);
                            }
                        }
                        let row = if self.params.len() >= 1 && self.params[0] > 0 {
                            (self.params[0] as usize).saturating_sub(1)
                        } else {
                            0
                        };
                        let col = if self.params.len() >= 2 && self.params[1] > 0 {
                            (self.params[1] as usize).saturating_sub(1)
                        } else {
                            0
                        };
                        screen.set_cursor(row, col);
                        self.state = ParserState::Ground;
                    }
                    b'J' => {
                        // Erase display
                        if !self.current_param.is_empty() {
                            if let Ok(param) = self.current_param.parse::<u16>() {
                                self.params.push(param);
                            }
                        }
                        let mode = if self.params.is_empty() { 0 } else { self.params[0] };
                        match mode {
                            0 => {
                                // Erase from cursor to end of screen
                                for row in screen.cursor_row..screen.rows {
                                    for col in screen.cursor_col..screen.cols {
                                        if col < screen.cols && row < screen.rows {
                                            screen.cells[row][col] = Default::default();
                                        }
                                    }
                                    screen.cursor_col = 0;
                                }
                            }
                            2 => {
                                // Erase entire screen
                                screen.clear();
                            }
                            _ => {}
                        }
                        self.state = ParserState::Ground;
                    }
                    b'K' => {
                        // Erase line
                        let mode = if self.params.is_empty() { 0 } else { self.params[0] };
                        match mode {
                            0 => {
                                // Erase from cursor to end of line
                                for col in screen.cursor_col..screen.cols {
                                    if col < screen.cols && screen.cursor_row < screen.rows {
                                        screen.cells[screen.cursor_row][col] = Default::default();
                                    }
                                }
                            }
                            2 => {
                                // Erase entire line
                                screen.clear_line(screen.cursor_row);
                            }
                            _ => {}
                        }
                        self.state = ParserState::Ground;
                    }
                    b'A' => {
                        // Cursor up
                        let count = if self.params.is_empty() || self.params[0] == 0 {
                            1
                        } else {
                            self.params[0] as usize
                        };
                        screen.cursor_row = screen.cursor_row.saturating_sub(count);
                        self.state = ParserState::Ground;
                    }
                    b'B' => {
                        // Cursor down
                        let count = if self.params.is_empty() || self.params[0] == 0 {
                            1
                        } else {
                            self.params[0] as usize
                        };
                        screen.cursor_row = (screen.cursor_row + count).min(screen.rows - 1);
                        self.state = ParserState::Ground;
                    }
                    b'C' => {
                        // Cursor forward (right)
                        let count = if self.params.is_empty() || self.params[0] == 0 {
                            1
                        } else {
                            self.params[0] as usize
                        };
                        screen.cursor_col = (screen.cursor_col + count).min(screen.cols - 1);
                        self.state = ParserState::Ground;
                    }
                    b'D' => {
                        // Cursor back (left)
                        let count = if self.params.is_empty() || self.params[0] == 0 {
                            1
                        } else {
                            self.params[0] as usize
                        };
                        screen.cursor_col = screen.cursor_col.saturating_sub(count);
                        self.state = ParserState::Ground;
                    }
                    b'r' => {
                        // Set scrolling region
                        if !self.current_param.is_empty() {
                            if let Ok(param) = self.current_param.parse::<u16>() {
                                self.params.push(param);
                            }
                        }
                        let top = if self.params.len() >= 1 && self.params[0] > 0 {
                            (self.params[0] as usize).saturating_sub(1)
                        } else {
                            0
                        };
                        let bottom = if self.params.len() >= 2 && self.params[1] > 0 {
                            (self.params[1] as usize).saturating_sub(1)
                        } else {
                            screen.rows - 1
                        };
                        screen.set_scroll_region(top, bottom);
                        self.state = ParserState::Ground;
                    }
                    _ => {
                        // Unknown CSI sequence
                        self.state = ParserState::Ground;
                    }
                }
            }
            ParserState::CSIParam => {
                // Handle intermediate characters (not implemented for MVP)
                if byte.is_ascii_digit() || byte == b';' {
                    if byte == b';' {
                        if !self.current_param.is_empty() {
                            if let Ok(param) = self.current_param.parse::<u16>() {
                                self.params.push(param);
                            }
                            self.current_param.clear();
                        }
                    } else {
                        self.current_param.push(byte as char);
                    }
                } else {
                    // Transition to final character
                    self.state = ParserState::CSI;
                    self.parse_byte(screen, byte);
                }
            }
            ParserState::OSC => {
                match byte {
                    0x07 | 0x1B => {
                        // OSC terminator (BEL or ESC)
                        self.osc_data.clear();
                        self.state = ParserState::Ground;
                    }
                    _ => {
                        if self.osc_data.len() < 1024 {
                            self.osc_data.push(byte as char);
                        }
                    }
                }
            }
        }
    }

    fn handle_sgr(&mut self, screen: &mut ScreenBuffer) {
        if self.params.is_empty() {
            // Reset all attributes
            screen.set_bold(false);
            screen.set_underline(false);
            screen.set_color(Color::Default, Color::Default);
            return;
        }

        for i in 0..self.params.len() {
            let param = self.params[i];
            match param {
                0 => {
                    // Reset
                    screen.set_bold(false);
                    screen.set_underline(false);
                    screen.set_color(Color::Default, Color::Default);
                }
                1 => {
                    // Bold
                    screen.set_bold(true);
                }
                4 => {
                    // Underline
                    screen.set_underline(true);
                }
                22 => {
                    // Not bold nor faint
                    screen.set_bold(false);
                }
                24 => {
                    // Not underlined
                    screen.set_underline(false);
                }
                30 => screen.set_color(Color::Indexed(0), Color::Default), // Black fg
                31 => screen.set_color(Color::Indexed(1), Color::Default), // Red fg
                32 => screen.set_color(Color::Indexed(2), Color::Default), // Green fg
                33 => screen.set_color(Color::Indexed(3), Color::Default), // Yellow fg
                34 => screen.set_color(Color::Indexed(4), Color::Default), // Blue fg
                35 => screen.set_color(Color::Indexed(5), Color::Default), // Magenta fg
                36 => screen.set_color(Color::Indexed(6), Color::Default), // Cyan fg
                37 => screen.set_color(Color::Indexed(7), Color::Default), // White fg
                39 => screen.set_color(Color::Default, Color::Default), // Default fg
                40 => screen.set_color(Color::Default, Color::Indexed(0)), // Black bg
                41 => screen.set_color(Color::Default, Color::Indexed(1)), // Red bg
                42 => screen.set_color(Color::Default, Color::Indexed(2)), // Green bg
                43 => screen.set_color(Color::Default, Color::Indexed(3)), // Yellow bg
                44 => screen.set_color(Color::Default, Color::Indexed(4)), // Blue bg
                45 => screen.set_color(Color::Default, Color::Indexed(5)), // Magenta bg
                46 => screen.set_color(Color::Default, Color::Indexed(6)), // Cyan bg
                47 => screen.set_color(Color::Default, Color::Indexed(7)), // White bg
                49 => screen.set_color(Color::Default, Color::Default), // Default bg
                _ => {}
            }
        }
    }
}
