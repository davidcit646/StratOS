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

/// Client-side title bar; explorer + terminal share the area below this strip.
pub const TITLE_BAR_HEIGHT_PX: u32 = FONT_HEIGHT as u32 + 8;

/// Horizontal rule between the upper explorer band and the PTY grid.
pub const SEPARATOR_PX: u32 = 2;

/// Pixels reserved below the scrolling file list for the preview block (`render_filesystem_panel`).
/// All list sizing and list-only hit tests go through [`explorer_list_band`].
pub const EXPLORER_PREVIEW_RESERVE_PX: u32 = 120;

/// File list rows only (excludes title line and preview); matches `render_filesystem_panel` layout.
#[derive(Clone, Copy, Debug)]
pub struct ExplorerListBand {
    /// First pixel row of the first list entry (after browser title line).
    pub list_top: u32,
    /// Same `max_list` cap as drawing (4..=32 rows).
    pub row_count: u32,
}

impl ExplorerListBand {
    /// Exclusive bottom of the list rows (where the inner list/preview separator begins).
    pub fn bottom_exclusive(self) -> u32 {
        self.list_top
            .saturating_add(self.row_count.saturating_mul(FONT_HEIGHT as u32))
    }
}

/// Shared geometry for the explorer file list vs preview split — keep in sync with `render_filesystem_panel`.
pub fn explorer_list_band(explorer_top: u32, explorer_bottom: u32) -> ExplorerListBand {
    let list_top = explorer_top
        .saturating_add(6)
        .saturating_add(FONT_HEIGHT as u32)
        .saturating_add(4);
    if explorer_bottom <= list_top {
        return ExplorerListBand {
            list_top,
            row_count: 4,
        };
    }
    let row_count = ((explorer_bottom
        .saturating_sub(list_top)
        .saturating_sub(EXPLORER_PREVIEW_RESERVE_PX))
        / FONT_HEIGHT as u32)
        .max(4)
        .min(32);
    ExplorerListBand { list_top, row_count }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentView {
    /// PTY / shell has keyboard focus (title bar highlights Terminal).
    Terminal,
    /// File explorer has keyboard focus (title bar highlights Files).
    Filesystem,
}

/// Pixel layout for the vertical split (title → explorer → separator → terminal).
#[derive(Clone, Copy, Debug)]
pub struct SplitLayout {
    /// Height of client-drawn title strip (0 when disabled in settings).
    pub title_bar_h: u32,
    pub buffer_width: u32,
    pub buffer_height: u32,
    pub explorer_top: u32,
    /// Y of the bar between explorer and terminal (`explorer_top` + explorer band height).
    pub separator_y: u32,
    pub terminal_top: u32,
    pub terminal_rows: u32,
}

impl SplitLayout {
    /// Lower ~3/5 of the content area is the PTY; upper ~2/5 is the explorer (minimum explorer height enforced when possible).
    pub fn compute(_window_width_px: i32, window_height_px: i32, cols: usize, client_title_bar: bool) -> Self {
        let fh = FONT_HEIGHT as u32;
        let fw = FONT_WIDTH as u32;
        let title_h = if client_title_bar {
            TITLE_BAR_HEIGHT_PX
        } else {
            0
        };
        let sep = SEPARATOR_PX;
        let h = window_height_px.max(1) as u32;
        let inner_h = h.saturating_sub(title_h);
        let inner = inner_h.saturating_sub(sep);

        let mut rows = (inner * 3 / (5 * fh)).max(1);
        let mut terminal_px = rows * fh;
        let mut explorer_h = inner.saturating_sub(terminal_px);

        const MIN_EXPLORER_H: u32 = FONT_HEIGHT as u32 * 5;
        if explorer_h < MIN_EXPLORER_H && rows > 1 {
            let target_rows = inner.saturating_sub(MIN_EXPLORER_H) / fh;
            rows = target_rows.max(1).min(rows);
            terminal_px = rows * fh;
            explorer_h = inner.saturating_sub(terminal_px);
        }

        let explorer_top = title_h;
        let separator_y = explorer_top + explorer_h;
        let terminal_top = separator_y + sep;

        SplitLayout {
            title_bar_h: title_h,
            buffer_width: (cols as u32).saturating_mul(fw),
            buffer_height: h,
            explorer_top,
            separator_y,
            terminal_top,
            terminal_rows: rows,
        }
    }

    pub fn list_band(&self) -> ExplorerListBand {
        explorer_list_band(self.explorer_top, self.separator_y)
    }
}

/// Pixel regions for pointer hit-testing (must match `draw_title_bar`).
#[derive(Clone, Debug)]
pub struct TitleBarRegions {
    pub files_tab: (u32, u32, u32, u32),
    pub terminal_tab: (u32, u32, u32, u32),
    /// Close sends `CloseWindow` from `title_bar_pick`.
    pub close_btn: (u32, u32, u32, u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleBarHit {
    FilesTab,
    TerminalTab,
    Close,
}

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

    /// Hit-test title bar chrome. `x`/`y` are surface coordinates in pixels.
    pub fn title_bar_pick(x: i32, y: i32, buffer_width: u32, title_bar_h: u32) -> Option<TitleBarHit> {
        if title_bar_h == 0 {
            return None;
        }
        if x < 0 || x >= buffer_width as i32 {
            return None;
        }
        if y < 0 || y >= title_bar_h as i32 {
            return None;
        }
        let regions = Self::title_bar_regions(buffer_width, title_bar_h);
        let xi = x as u32;
        let yi = y as u32;
        if rect_hit(regions.files_tab, xi, yi) {
            return Some(TitleBarHit::FilesTab);
        }
        if rect_hit(regions.terminal_tab, xi, yi) {
            return Some(TitleBarHit::TerminalTab);
        }
        if rect_hit(regions.close_btn, xi, yi) {
            return Some(TitleBarHit::Close);
        }
        None
    }

    pub fn title_bar_regions(buffer_width: u32, title_bar_h: u32) -> TitleBarRegions {
        let fw = FONT_WIDTH as u32;
        let _fh = FONT_HEIGHT as u32;
        let tab_y = 3u32;
        let bar_h = title_bar_h.max(1);
        let mut x = 8u32 + 10 * fw + 12;
        let pad_x = 6u32;
        let files_w = 7 * fw + pad_x * 2;
        let files_tab = (x, tab_y.saturating_sub(1), files_w, bar_h.saturating_sub(2));
        x += files_w + 8;
        let term_w = 10 * fw + pad_x * 2;
        let terminal_tab = (x, tab_y.saturating_sub(1), term_w, bar_h.saturating_sub(2));
        let btn_w = fw * 2 + 4;
        let close_x = buffer_width.saturating_sub(btn_w + 8);
        let close_btn = (close_x, 4, btn_w, FONT_HEIGHT as u32 + 4);
        TitleBarRegions {
            files_tab,
            terminal_tab,
            close_btn,
        }
    }

    pub fn render(
        &mut self,
        screen: &ScreenBuffer,
        window: &mut WaylandWindow,
        overlay: Option<&UiOverlay>,
        layout: &SplitLayout,
        focus: ContentView,
    ) -> Result<(), String> {
        let cell_width = FONT_WIDTH as u32;
        let cell_height = FONT_HEIGHT as u32;

        let buffer_width = layout.buffer_width;
        let buffer_height = layout.buffer_height;

        let mut buffer = vec![0u8; (buffer_width * buffer_height * 4) as usize];

        let y_off = layout.terminal_top;

        if let Some(ui) = overlay {
            self.render_filesystem_panel(
                &mut buffer,
                buffer_width,
                layout.explorer_top,
                layout.separator_y,
                ui,
            );
        }

        draw_rect(
            &mut buffer,
            buffer_width,
            0,
            layout.separator_y,
            buffer_width,
            SEPARATOR_PX,
            (56, 62, 80),
        );

        for row_idx in 0..screen.rows {
            for col_idx in 0..screen.cols {
                let cell = screen.display_cell(row_idx, col_idx);

                let cell_x = col_idx as u32 * cell_width;
                let cell_y = y_off + row_idx as u32 * cell_height;

                let (fg_r, fg_g, fg_b) = color_to_rgb(cell.fg);
                let (bg_r, bg_g, bg_b) = bg_color_to_rgb(cell.bg);

                let (fg_r, fg_g, fg_b) = if cell.bold {
                    ((fg_r as u16 + 50).min(255) as u8,
                     (fg_g as u16 + 50).min(255) as u8,
                     (fg_b as u16 + 50).min(255) as u8)
                } else {
                    (fg_r, fg_g, fg_b)
                };

                let ch_idx = cell.ch as usize;
                let glyph = if ch_idx < 128 {
                    FONT_DATA[ch_idx]
                } else {
                    FONT_DATA[0]
                };

                for yy in 0..cell_height {
                    let pixel_row = (cell_y + yy) as usize;
                    if pixel_row >= buffer_height as usize {
                        continue;
                    }
                    let glyph_row = glyph[yy as usize];
                    for xx in 0..cell_width {
                        let pixel_col = (cell_x + xx) as usize;
                        if pixel_col >= buffer_width as usize {
                            continue;
                        }
                        let glyph_bit = (glyph_row >> (7 - xx)) & 1;
                        let pixel_idx = (pixel_row * buffer_width as usize + pixel_col) * 4;
                        if glyph_bit == 1 {
                            buffer[pixel_idx] = fg_b;
                            buffer[pixel_idx + 1] = fg_g;
                            buffer[pixel_idx + 2] = fg_r;
                        } else {
                            buffer[pixel_idx] = bg_b;
                            buffer[pixel_idx + 1] = bg_g;
                            buffer[pixel_idx + 2] = bg_r;
                        }
                        buffer[pixel_idx + 3] = 255;
                    }
                }

                if cell.underline {
                    let underline_y = (cell_y + cell_height - 1) as usize;
                    if underline_y < buffer_height as usize {
                        for xx in 0..cell_width {
                            let pixel_col = (cell_x + xx) as usize;
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

        if !screen.is_scrollback_active() {
            let cursor_x = screen.cursor_col as u32 * cell_width;
            let cursor_y = y_off + screen.cursor_row as u32 * cell_height;
            for yy in 0..cell_height {
                let row_idx = (cursor_y + yy) as usize;
                if row_idx >= buffer_height as usize {
                    continue;
                }
                for xx in 0..cell_width {
                    let col_idx = (cursor_x + xx) as usize;
                    if col_idx >= buffer_width as usize {
                        continue;
                    }
                    let pixel_idx = (row_idx * buffer_width as usize + col_idx) * 4;
                    buffer[pixel_idx] = 255 - buffer[pixel_idx];
                    buffer[pixel_idx + 1] = 255 - buffer[pixel_idx + 1];
                    buffer[pixel_idx + 2] = 255 - buffer[pixel_idx + 2];
                }
            }
        }

        if let Some(ui) = overlay {
            if !ui.status_chip.is_empty() {
                self.render_status_chip(
                    &mut buffer,
                    buffer_width,
                    layout.terminal_top.saturating_add(8),
                    &ui.status_chip,
                );
            }
            if !ui.ghost_suffix.is_empty() {
                self.render_ghost_text(
                    &mut buffer,
                    buffer_width,
                    y_off,
                    screen,
                    &ui.ghost_suffix,
                );
            }
        }

        if layout.title_bar_h > 0 {
            if let Some(ui) = overlay {
                draw_title_bar(&mut buffer, buffer_width, focus, ui);
            } else {
                draw_title_bar_minimal(&mut buffer, buffer_width, focus);
            }
        }

        let (out_w, padded) = pad_rgba_right_edge(buffer, buffer_width, self.width, buffer_height);
        window.render_buffer(&padded, out_w, buffer_height)
    }

    fn render_status_chip(&self, buffer: &mut [u8], buffer_width: u32, chip_y: u32, text: &str) {
        if text.is_empty() {
            return;
        }
        let chars = text.chars().count().min(72) as u32;
        let width = (chars + 2) * FONT_WIDTH as u32;
        let height = (FONT_HEIGHT as u32) + 4;
        draw_rect(buffer, buffer_width, 4, chip_y, width, height, (34, 64, 120));
        draw_text(
            buffer,
            buffer_width,
            8,
            chip_y + 2,
            text,
            (245, 245, 245),
            (34, 64, 120),
        );
    }

    fn render_filesystem_panel(
        &self,
        buffer: &mut [u8],
        buffer_width: u32,
        explorer_top: u32,
        explorer_bottom: u32,
        ui: &UiOverlay,
    ) {
        if explorer_bottom <= explorer_top {
            return;
        }
        draw_rect(
            buffer,
            buffer_width,
            0,
            explorer_top,
            buffer_width,
            explorer_bottom.saturating_sub(explorer_top),
            (18, 20, 28),
        );

        let panel_x = 0u32;
        let panel_width = buffer_width;
        let band = explorer_list_band(explorer_top, explorer_bottom);
        let max_list = band.row_count;
        let mut y = explorer_top + 6;

        draw_text(
            buffer,
            buffer_width,
            panel_x + 8,
            y,
            &ui.browser_title,
            (180, 210, 255),
            (18, 20, 28),
        );
        y = band.list_top;

        for line in ui.browser_lines.iter().take(max_list as usize) {
            let selected = line.starts_with('>');
            if selected {
                draw_rect(
                    buffer,
                    buffer_width,
                    panel_x,
                    y.saturating_sub(1),
                    panel_width,
                    (FONT_HEIGHT as u32) + 2,
                    (34, 48, 78),
                );
            }
            draw_text(
                buffer,
                buffer_width,
                panel_x + 8,
                y,
                line,
                if selected { (255, 255, 255) } else { (235, 235, 235) },
                if selected { (34, 48, 78) } else { (18, 20, 28) },
            );
            y += FONT_HEIGHT as u32;
        }

        let split_top = y;
        y += 4;
        draw_rect(
            buffer,
            buffer_width,
            panel_x,
            split_top,
            panel_width,
            2,
            (56, 62, 80),
        );
        let preview_bg = (26, 28, 38);
        let preview_h = explorer_bottom.saturating_sub(y);
        draw_rect(
            buffer,
            buffer_width,
            panel_x,
            y,
            panel_width,
            preview_h,
            preview_bg,
        );
        draw_text(
            buffer,
            buffer_width,
            panel_x + 8,
            y,
            &ui.preview_title,
            (255, 205, 145),
            preview_bg,
        );
        y += (FONT_HEIGHT as u32) + 2;
        for line in ui.preview_lines.iter().take(16) {
            if y >= explorer_bottom.saturating_sub(FONT_HEIGHT as u32) {
                break;
            }
            draw_text(
                buffer,
                buffer_width,
                panel_x + 8,
                y,
                line,
                (210, 210, 210),
                preview_bg,
            );
            y += FONT_HEIGHT as u32;
        }
    }

    fn render_ghost_text(
        &self,
        buffer: &mut [u8],
        buffer_width: u32,
        y_off: u32,
        screen: &ScreenBuffer,
        ghost_suffix: &str,
    ) {
        let x = (screen.cursor_col as u32) * (FONT_WIDTH as u32);
        let y = y_off + (screen.cursor_row as u32) * (FONT_HEIGHT as u32);
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

fn rect_hit(r: (u32, u32, u32, u32), x: u32, y: u32) -> bool {
    let (rx, ry, rw, rh) = r;
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

fn draw_title_bar_minimal(
    buffer: &mut [u8],
    buffer_width: u32,
    active: ContentView,
) {
    draw_rect(
        buffer,
        buffer_width,
        0,
        0,
        buffer_width,
        TITLE_BAR_HEIGHT_PX,
        (40, 42, 52),
    );
    draw_rect(
        buffer,
        buffer_width,
        0,
        TITLE_BAR_HEIGHT_PX.saturating_sub(2),
        buffer_width,
        2,
        (70, 74, 88),
    );
    draw_title_bar_inner(buffer, buffer_width, active, None);
}

fn draw_title_bar(
    buffer: &mut [u8],
    buffer_width: u32,
    active: ContentView,
    _ui: &UiOverlay,
) {
    draw_title_bar_minimal(buffer, buffer_width, active);
}

fn draw_title_bar_inner(
    buffer: &mut [u8],
    buffer_width: u32,
    active: ContentView,
    _subtitle: Option<&str>,
) {
    let fw = FONT_WIDTH as u32;
    let regions = Renderer::title_bar_regions(buffer_width, TITLE_BAR_HEIGHT_PX);
    let tab_y = 3u32;

    draw_text(
        buffer,
        buffer_width,
        8,
        tab_y,
        "StratTerm",
        (220, 220, 220),
        (40, 42, 52),
    );

    let (fx, fy, fwid, fhgt) = regions.files_tab;
    let files_active = active == ContentView::Filesystem;
    draw_rect(
        buffer,
        buffer_width,
        fx,
        fy,
        fwid,
        fhgt,
        if files_active {
            (52, 68, 110)
        } else {
            (48, 50, 62)
        },
    );
    draw_text(
        buffer,
        buffer_width,
        fx + 6,
        tab_y,
        " Files ",
        (255, 255, 255),
        if files_active {
            (52, 68, 110)
        } else {
            (48, 50, 62)
        },
    );

    let (tx, ty, twid, thgt) = regions.terminal_tab;
    let term_active = active == ContentView::Terminal;
    draw_rect(
        buffer,
        buffer_width,
        tx,
        ty,
        twid,
        thgt,
        if term_active {
            (52, 68, 110)
        } else {
            (48, 50, 62)
        },
    );
    draw_text(
        buffer,
        buffer_width,
        tx + 6,
        tab_y,
        " Terminal ",
        (255, 255, 255),
        if term_active {
            (52, 68, 110)
        } else {
            (48, 50, 62)
        },
    );

    let (cx, _, _, _) = regions.close_btn;
    let mut bx = cx.saturating_sub(fw * 11);
    for (label, bg) in [
        (" _ ", (48u8, 50u8, 62u8)),
        (" [] ", (48u8, 50u8, 62u8)),
    ] {
        draw_text(
            buffer,
            buffer_width,
            bx,
            tab_y,
            label,
            (200, 200, 200),
            bg,
        );
        bx += (label.chars().count() as u32) * fw;
    }

    let (cx, cy, cw, ch) = regions.close_btn;
    draw_rect(buffer, buffer_width, cx, cy, cw, ch, (120, 50, 55));
    draw_text(
        buffer,
        buffer_width,
        cx + cw / 2 - fw / 2,
        tab_y,
        "X",
        (255, 255, 255),
        (120, 50, 55),
    );
}

/// When the Wayland surface is wider than `cols * FONT_WIDTH`, extend each row with opaque black (BGR888 + A).
fn pad_rgba_right_edge(
    src: Vec<u8>,
    src_w: u32,
    dst_w: u32,
    height: u32,
) -> (u32, Vec<u8>) {
    if dst_w <= src_w {
        return (src_w, src);
    }
    let mut out = vec![0u8; (dst_w as usize) * (height as usize) * 4];
    let src_stride = src_w as usize * 4;
    let dst_stride = dst_w as usize * 4;
    for row in 0..height {
        let s = row as usize * src_stride;
        let d = row as usize * dst_stride;
        out[d..d + src_stride].copy_from_slice(&src[s..s + src_stride]);
        for x in src_w..dst_w {
            let i = d + x as usize * 4;
            out[i] = 0;
            out[i + 1] = 0;
            out[i + 2] = 0;
            out[i + 3] = 255;
        }
    }
    (dst_w, out)
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
