use crate::font::{FONT_DATA, FONT_HEIGHT, FONT_WIDTH};
use crate::screen::{Color, ScreenBuffer};
use crate::wayland::WaylandWindow;

// Standard 16-color ANSI palette
const ANSI_COLORS: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0: Black
    (205, 0, 0),    // 1: Red
    (0, 205, 0),    // 2: Green
    (205, 205, 0),  // 3: Yellow
    (0, 0, 238),    // 4: Blue
    (205, 0, 205),  // 5: Magenta
    (0, 205, 205),  // 6: Cyan
    (229, 229, 229), // 7: White
    (127, 127, 127), // 8: Bright Black (Gray)
    (255, 0, 0),    // 9: Bright Red
    (0, 255, 0),    // 10: Bright Green
    (255, 255, 0),  // 11: Bright Yellow
    (92, 92, 255),  // 12: Bright Blue
    (255, 0, 255),  // 13: Bright Magenta
    (0, 255, 255),  // 14: Bright Cyan
    (255, 255, 255), // 15: Bright White
];

fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Default => (229, 229, 229), // Default foreground (light gray)
        Color::Indexed(idx) => {
            let idx = idx as usize % 16;
            ANSI_COLORS[idx]
        }
        Color::RGB(r, g, b) => (r, g, b),
    }
}

fn bg_color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Default => (0, 0, 0), // Default background (black)
        Color::Indexed(idx) => {
            let idx = idx as usize % 16;
            ANSI_COLORS[idx]
        }
        Color::RGB(r, g, b) => (r, g, b),
    }
}

pub struct Renderer {
    width: u32,
    height: u32,
}

#[derive(Clone, Debug, Default)]
pub struct UiOverlay {
    pub status_chip: String,
    pub browser_active: bool,
    pub browser_title: String,
    pub browser_lines: Vec<String>,
    pub preview_title: String,
    pub preview_lines: Vec<String>,
    pub ghost_suffix: String,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Renderer {
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn render(
        &mut self,
        screen: &ScreenBuffer,
        window: &mut WaylandWindow,
        overlay: Option<&UiOverlay>,
    ) -> Result<(), String> {
        let cell_width = FONT_WIDTH as u32;
        let cell_height = FONT_HEIGHT as u32;
        
        let cols = screen.cols as u32;
        let rows = screen.rows as u32;
        
        // Calculate buffer size
        let buffer_width = cols * cell_width;
        let buffer_height = rows * cell_height;
        
        // Create ARGB8888 buffer
        let mut buffer = vec![0u8; (buffer_width * buffer_height * 4) as usize];
        
        // Render each cell
        for row_idx in 0..screen.rows {
            for col_idx in 0..screen.cols {
                let cell = screen.display_cell(row_idx, col_idx);
                
                let cell_x = col_idx as u32 * cell_width;
                let cell_y = row_idx as u32 * cell_height;
                
                // Get colors
                let (fg_r, fg_g, fg_b) = color_to_rgb(cell.fg);
                let (bg_r, bg_g, bg_b) = bg_color_to_rgb(cell.bg);
                
                // Apply bold by brightening
                let (fg_r, fg_g, fg_b) = if cell.bold {
                    ((fg_r as u16 + 50).min(255) as u8,
                     (fg_g as u16 + 50).min(255) as u8,
                     (fg_b as u16 + 50).min(255) as u8)
                } else {
                    (fg_r, fg_g, fg_b)
                };
                
                // Get glyph bitmap
                let ch_idx = cell.ch as usize;
                let glyph = if ch_idx < 128 {
                    FONT_DATA[ch_idx]
                } else {
                    FONT_DATA[0] // Use NUL for unknown chars
                };
                
                // Render cell
                for y in 0..cell_height {
                    let pixel_row = (cell_y + y) as usize;
                    if pixel_row >= buffer_height as usize {
                        continue;
                    }
                    
                    let glyph_row = glyph[y as usize];
                    
                    for x in 0..cell_width {
                        let pixel_col = (cell_x + x) as usize;
                        if pixel_col >= buffer_width as usize {
                            continue;
                        }
                        
                        let pixel_idx = (pixel_row * buffer_width as usize + pixel_col) * 4;
                        
                        // Check if pixel is set in glyph
                        let glyph_bit = (glyph_row >> (7 - x)) & 1;
                        
                        if glyph_bit == 1 {
                            // Foreground
                            buffer[pixel_idx] = fg_b;     // B
                            buffer[pixel_idx + 1] = fg_g; // G
                            buffer[pixel_idx + 2] = fg_r; // R
                            buffer[pixel_idx + 3] = 255; // A
                        } else {
                            // Background
                            buffer[pixel_idx] = bg_b;     // B
                            buffer[pixel_idx + 1] = bg_g; // G
                            buffer[pixel_idx + 2] = bg_r; // R
                            buffer[pixel_idx + 3] = 255; // A
                        }
                    }
                }
                
                // Render underline
                if cell.underline {
                    let underline_y = (cell_y + cell_height - 1) as usize;
                    if underline_y < buffer_height as usize {
                        for x in 0..cell_width {
                            let pixel_col = (cell_x + x) as usize;
                            if pixel_col >= buffer_width as usize {
                                continue;
                            }
                            
                            let pixel_idx = (underline_y * buffer_width as usize + pixel_col) * 4;
                            buffer[pixel_idx] = fg_b;
                            buffer[pixel_idx + 1] = fg_g;
                            buffer[pixel_idx + 2] = fg_r;
                            buffer[pixel_idx + 3] = 255;
                        }
                    }
                }
            }
        }
        
        // Render cursor only in live viewport mode.
        if !screen.is_scrollback_active() {
            let cursor_x = screen.cursor_col as u32 * cell_width;
            let cursor_y = screen.cursor_row as u32 * cell_height;

            for y in 0..cell_height {
                let row_idx = (cursor_y + y) as usize;
                if row_idx >= buffer_height as usize {
                    continue;
                }

                for x in 0..cell_width {
                    let col_idx = (cursor_x + x) as usize;
                    if col_idx >= buffer_width as usize {
                        continue;
                    }

                    let pixel_idx = (row_idx * buffer_width as usize + col_idx) * 4;

                    // Invert cursor colors
                    buffer[pixel_idx] = 255 - buffer[pixel_idx];
                    buffer[pixel_idx + 1] = 255 - buffer[pixel_idx + 1];
                    buffer[pixel_idx + 2] = 255 - buffer[pixel_idx + 2];
                }
            }
        }
        
        if let Some(ui) = overlay {
            self.render_status_chip(&mut buffer, buffer_width, &ui.status_chip);
            if ui.browser_active {
                self.render_browser_overlay(&mut buffer, buffer_width, buffer_height, ui);
            }
            if !ui.ghost_suffix.is_empty() {
                self.render_ghost_text(&mut buffer, buffer_width, screen, &ui.ghost_suffix);
            }
        }

        // Send buffer to Wayland window
        window.render_buffer(&buffer, buffer_width, buffer_height)
    }

    fn render_status_chip(&self, buffer: &mut [u8], buffer_width: u32, text: &str) {
        if text.is_empty() {
            return;
        }
        let chars = text.chars().count().min(72) as u32;
        let width = (chars + 2) * FONT_WIDTH as u32;
        let height = (FONT_HEIGHT as u32) + 4;
        draw_rect(buffer, buffer_width, 4, 4, width, height, (34, 64, 120));
        draw_text(buffer, buffer_width, 8, 6, text, (245, 245, 245), (34, 64, 120));
    }

    fn render_browser_overlay(
        &self,
        buffer: &mut [u8],
        buffer_width: u32,
        buffer_height: u32,
        ui: &UiOverlay,
    ) {
        let panel_width = (buffer_width / 3).max((FONT_WIDTH as u32) * 36);
        let panel_x = buffer_width.saturating_sub(panel_width);
        draw_rect(
            buffer,
            buffer_width,
            panel_x,
            0,
            panel_width,
            buffer_height,
            (18, 20, 28),
        );

        let mut y = 6u32;
        draw_text(
            buffer,
            buffer_width,
            panel_x + 8,
            y,
            &ui.browser_title,
            (180, 210, 255),
            (18, 20, 28),
        );
        y += (FONT_HEIGHT as u32) + 4;

        for line in ui.browser_lines.iter().take(16) {
            draw_text(
                buffer,
                buffer_width,
                panel_x + 8,
                y,
                line,
                (235, 235, 235),
                (18, 20, 28),
            );
            y += FONT_HEIGHT as u32;
        }

        y += 4;
        draw_text(
            buffer,
            buffer_width,
            panel_x + 8,
            y,
            &ui.preview_title,
            (255, 205, 145),
            (18, 20, 28),
        );
        y += (FONT_HEIGHT as u32) + 2;
        for line in ui.preview_lines.iter().take(12) {
            draw_text(
                buffer,
                buffer_width,
                panel_x + 8,
                y,
                line,
                (210, 210, 210),
                (18, 20, 28),
            );
            y += FONT_HEIGHT as u32;
        }
    }

    fn render_ghost_text(
        &self,
        buffer: &mut [u8],
        buffer_width: u32,
        screen: &ScreenBuffer,
        ghost_suffix: &str,
    ) {
        let x = (screen.cursor_col as u32) * (FONT_WIDTH as u32);
        let y = (screen.cursor_row as u32) * (FONT_HEIGHT as u32);
        draw_text(
            buffer,
            buffer_width,
            x,
            y,
            ghost_suffix,
            (120, 120, 120),
            (0, 0, 0),
        );
    }
}

fn draw_rect(
    buffer: &mut [u8],
    buffer_width: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: (u8, u8, u8),
) {
    let (r, g, b) = color;
    let stride = buffer_width as usize * 4;
    let max_height = buffer.len() / stride;
    let end_y = (y + height).min(max_height as u32);
    let end_x = (x + width).min(buffer_width);

    for row in y..end_y {
        let row_base = row as usize * stride;
        for col in x..end_x {
            let index = row_base + (col as usize * 4);
            buffer[index] = b;
            buffer[index + 1] = g;
            buffer[index + 2] = r;
            buffer[index + 3] = 255;
        }
    }
}

fn draw_text(
    buffer: &mut [u8],
    buffer_width: u32,
    x: u32,
    y: u32,
    text: &str,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) {
    let mut cursor_x = x;
    for ch in text.chars().take(96) {
        draw_glyph(buffer, buffer_width, cursor_x, y, ch, fg, bg);
        cursor_x += FONT_WIDTH as u32;
    }
}

fn draw_glyph(
    buffer: &mut [u8],
    buffer_width: u32,
    x: u32,
    y: u32,
    ch: char,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) {
    let stride = buffer_width as usize * 4;
    let max_height = buffer.len() / stride;
    let glyph_index = (ch as usize).min(127);
    let glyph = FONT_DATA[glyph_index];

    for row in 0..FONT_HEIGHT as u32 {
        let py = y + row;
        if py as usize >= max_height {
            continue;
        }
        let row_base = py as usize * stride;
        let bits = glyph[row as usize];
        for col in 0..FONT_WIDTH as u32 {
            let px = x + col;
            if px >= buffer_width {
                continue;
            }
            let set = ((bits >> (7 - col)) & 1) == 1;
            let (r, g, b) = if set { fg } else { bg };
            let index = row_base + (px as usize * 4);
            buffer[index] = b;
            buffer[index + 1] = g;
            buffer[index + 2] = r;
            buffer[index + 3] = 255;
        }
    }
}
