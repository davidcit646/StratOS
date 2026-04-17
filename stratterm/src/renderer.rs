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

    pub fn render(&mut self, screen: &ScreenBuffer, window: &mut WaylandWindow) -> Result<(), String> {
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
                let cell = &screen.cells[row_idx][col_idx];
                
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
        
        // Render cursor
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
        
        // Send buffer to Wayland window
        window.render_buffer(&buffer, buffer_width, buffer_height)
    }
}
