use crate::font::{FONT_DATA, FONT_HEIGHT, FONT_WIDTH};
use crate::screen::{Color, ScreenBuffer};
use crate::wayland::WaylandWindow;

// Standard 16-color ANSI palette tuned for a cooler modern terminal theme.
const ANSI_COLORS: [(u8, u8, u8); 16] = [
    (17, 24, 39),    // 0: Black
    (239, 83, 80),   // 1: Red
    (94, 194, 113),  // 2: Green
    (244, 181, 71),  // 3: Yellow
    (96, 165, 250),  // 4: Blue
    (236, 72, 153),  // 5: Magenta
    (56, 189, 248),  // 6: Cyan
    (214, 223, 237), // 7: White
    (107, 114, 128), // 8: Bright Black (Gray)
    (255, 120, 117), // 9: Bright Red
    (134, 239, 172), // 10: Bright Green
    (250, 204, 21),  // 11: Bright Yellow
    (125, 211, 252), // 12: Bright Blue
    (244, 114, 182), // 13: Bright Magenta
    (103, 232, 249), // 14: Bright Cyan
    (248, 250, 252), // 15: Bright White
];

const TERM_FG_DEFAULT: (u8, u8, u8) = (226, 232, 240);
const TERM_BG_DEFAULT: (u8, u8, u8) = (11, 15, 24);

const UI_TITLE_BG: (u8, u8, u8) = (20, 27, 43);
const UI_TITLE_RULE: (u8, u8, u8) = (46, 58, 88);
const UI_SURFACE: (u8, u8, u8) = (16, 22, 35);
const UI_SURFACE_ALT: (u8, u8, u8) = (20, 28, 44);
const UI_PANEL_BORDER: (u8, u8, u8) = (44, 58, 86);
const UI_ACCENT: (u8, u8, u8) = (108, 184, 255);
const UI_ACCENT_SOFT: (u8, u8, u8) = (42, 70, 114);
const UI_TEXT: (u8, u8, u8) = (233, 240, 252);
const UI_TEXT_MUTED: (u8, u8, u8) = (154, 170, 198);
const UI_CLOSE_BG: (u8, u8, u8) = (162, 63, 66);

/// Client-side title bar; explorer + terminal share the area below this strip.
const TITLE_BAR_HEIGHT_BASE_PX: u32 = FONT_HEIGHT as u32 + 12;
/// Horizontal rule between the upper explorer band and the PTY grid.
const SEPARATOR_BASE_PX: u32 = 2;
/// Pixels reserved below the scrolling file list for the preview block (`render_filesystem_panel`).
const EXPLORER_PREVIEW_RESERVE_BASE_PX: u32 = 132;

fn quantize_scale(scale: f32) -> u32 {
    if !scale.is_finite() {
        return 1;
    }
    scale.clamp(1.0, 4.0).round() as u32
}

#[inline]
fn scaled_px(px: u32, scale: u32) -> u32 {
    px.saturating_mul(scale.max(1))
}

#[derive(Clone, Copy, Debug)]
pub struct RenderScale {
    pub terminal: u32,
    pub ui: u32,
}

impl RenderScale {
    pub fn from_settings(terminal_scale: f32, ui_scale: f32) -> Self {
        RenderScale {
            terminal: quantize_scale(terminal_scale),
            ui: quantize_scale(ui_scale),
        }
    }

    pub fn terminal_cell_width(self) -> u32 {
        FONT_WIDTH as u32 * self.terminal.max(1)
    }

    pub fn terminal_cell_height(self) -> u32 {
        FONT_HEIGHT as u32 * self.terminal.max(1)
    }

    pub fn ui_cell_width(self) -> u32 {
        FONT_WIDTH as u32 * self.ui.max(1)
    }

    pub fn ui_cell_height(self) -> u32 {
        FONT_HEIGHT as u32 * self.ui.max(1)
    }
}

/// File list rows only (excludes title line and preview); matches `render_filesystem_panel` layout.
#[derive(Clone, Copy, Debug)]
pub struct ExplorerListBand {
    /// First pixel row of the first list entry (after browser title line).
    pub list_top: u32,
    /// Same `max_list` cap as drawing (4..=32 rows).
    pub row_count: u32,
    /// Height in pixels of a single row.
    pub row_height: u32,
}

impl ExplorerListBand {
    /// Exclusive bottom of the list rows (where the inner list/preview separator begins).
    pub fn bottom_exclusive(self) -> u32 {
        self.list_top
            .saturating_add(self.row_count.saturating_mul(self.row_height))
    }
}

/// Shared geometry for the explorer file list vs preview split — keep in sync with `render_filesystem_panel`.
pub fn explorer_list_band(
    explorer_top: u32,
    explorer_bottom: u32,
    row_height: u32,
    ui_scale: u32,
) -> ExplorerListBand {
    let row_height = row_height.max(1);
    let list_top = explorer_top
        .saturating_add(scaled_px(6, ui_scale))
        .saturating_add(row_height)
        .saturating_add(scaled_px(4, ui_scale));
    if explorer_bottom <= list_top {
        return ExplorerListBand {
            list_top,
            row_count: 4,
            row_height,
        };
    }
    let row_count = ((explorer_bottom
        .saturating_sub(list_top)
        .saturating_sub(scaled_px(EXPLORER_PREVIEW_RESERVE_BASE_PX, ui_scale)))
        / row_height)
        .max(4)
        .min(32);
    ExplorerListBand {
        list_top,
        row_count,
        row_height,
    }
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
    pub separator_h: u32,
    pub buffer_width: u32,
    pub buffer_height: u32,
    pub explorer_top: u32,
    /// Y of the bar between explorer and terminal (`explorer_top` + explorer band height).
    pub separator_y: u32,
    pub terminal_top: u32,
    pub terminal_rows: u32,
    pub term_cell_width: u32,
    pub term_cell_height: u32,
    pub ui_cell_width: u32,
    pub ui_cell_height: u32,
    pub ui_scale: u32,
}

impl SplitLayout {
    /// Lower ~3/5 of the content area is the PTY; upper ~2/5 is the explorer (minimum explorer height enforced when possible).
    pub fn compute(
        _window_width_px: i32,
        window_height_px: i32,
        cols: usize,
        client_title_bar: bool,
        scale: RenderScale,
    ) -> Self {
        let fh = scale.terminal_cell_height();
        let fw = scale.terminal_cell_width();
        let ui_h = scale.ui_cell_height();
        let ui_w = scale.ui_cell_width();
        let title_h = if client_title_bar {
            scaled_px(TITLE_BAR_HEIGHT_BASE_PX, scale.ui)
        } else {
            0
        };
        let sep = scaled_px(SEPARATOR_BASE_PX, scale.ui).max(1);
        let h = window_height_px.max(1) as u32;
        let inner_h = h.saturating_sub(title_h);
        let inner = inner_h.saturating_sub(sep);

        let mut rows = (inner * 3 / (5 * fh)).max(1);
        let mut terminal_px = rows * fh;
        let mut explorer_h = inner.saturating_sub(terminal_px);

        let min_explorer_h: u32 = ui_h * 5;
        if explorer_h < min_explorer_h && rows > 1 {
            let target_rows = inner.saturating_sub(min_explorer_h) / fh;
            rows = target_rows.max(1).min(rows);
            terminal_px = rows * fh;
            explorer_h = inner.saturating_sub(terminal_px);
        }

        let explorer_top = title_h;
        let separator_y = explorer_top + explorer_h;
        let terminal_top = separator_y + sep;

        SplitLayout {
            title_bar_h: title_h,
            separator_h: sep,
            buffer_width: (cols as u32).saturating_mul(fw),
            buffer_height: h,
            explorer_top,
            separator_y,
            terminal_top,
            terminal_rows: rows,
            term_cell_width: fw,
            term_cell_height: fh,
            ui_cell_width: ui_w,
            ui_cell_height: ui_h,
            ui_scale: scale.ui,
        }
    }

    pub fn list_band(&self) -> ExplorerListBand {
        explorer_list_band(
            self.explorer_top,
            self.separator_y,
            self.ui_cell_height,
            self.ui_scale,
        )
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
        Color::Default => TERM_FG_DEFAULT, // Default foreground
        Color::Indexed(idx) => {
            let idx = idx as usize % 16;
            ANSI_COLORS[idx]
        }
        Color::RGB(r, g, b) => (r, g, b),
    }
}

fn bg_color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Default => TERM_BG_DEFAULT, // Default background
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
        Renderer { width, height }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Hit-test title bar chrome. `x`/`y` are surface coordinates in pixels.
    pub fn title_bar_pick(
        x: i32,
        y: i32,
        layout: &SplitLayout,
    ) -> Option<TitleBarHit> {
        let buffer_width = layout.buffer_width;
        let title_bar_h = layout.title_bar_h;
        if title_bar_h == 0 {
            return None;
        }
        if x < 0 || x >= buffer_width as i32 {
            return None;
        }
        if y < 0 || y >= title_bar_h as i32 {
            return None;
        }
        let regions = Self::title_bar_regions(layout);
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

    pub fn title_bar_regions(layout: &SplitLayout) -> TitleBarRegions {
        let buffer_width = layout.buffer_width;
        let title_bar_h = layout.title_bar_h;
        let fw = layout.ui_cell_width;
        let ui = layout.ui_scale;
        let tab_y = scaled_px(4, ui);
        let bar_h = title_bar_h.max(1);
        let tab_h = bar_h
            .saturating_sub(scaled_px(8, ui))
            .max(layout.ui_cell_height + scaled_px(2, ui));
        let mut x = scaled_px(10, ui) + 10 * fw + scaled_px(16, ui);
        let pad_x = scaled_px(8, ui);
        let files_w = 5 * fw + pad_x * 2;
        let files_tab = (x, tab_y, files_w, tab_h);
        x += files_w + scaled_px(8, ui);
        let term_w = 8 * fw + pad_x * 2;
        let terminal_tab = (x, tab_y, term_w, tab_h);
        let btn_w = fw * 2 + scaled_px(10, ui);
        let close_x = buffer_width.saturating_sub(btn_w + scaled_px(10, ui));
        let close_btn = (close_x, tab_y, btn_w, tab_h);
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
        let cell_width = layout.term_cell_width;
        let cell_height = layout.term_cell_height;

        let buffer_width = layout.buffer_width;
        let buffer_height = layout.buffer_height;

        let mut buffer = vec![0u8; (buffer_width * buffer_height * 4) as usize];

        let y_off = layout.terminal_top;

        if let Some(ui) = overlay {
            self.render_filesystem_panel(
                &mut buffer,
                layout,
                ui,
            );
        }

        draw_rect(
            &mut buffer,
            buffer_width,
            0,
            layout.separator_y,
            buffer_width,
            layout.separator_h,
            UI_TITLE_RULE,
        );

        for row_idx in 0..screen.rows {
            for col_idx in 0..screen.cols {
                let cell = screen.display_cell(row_idx, col_idx);

                let cell_x = col_idx as u32 * cell_width;
                let cell_y = y_off + row_idx as u32 * cell_height;

                let (fg_r, fg_g, fg_b) = color_to_rgb(cell.fg);
                let (bg_r, bg_g, bg_b) = bg_color_to_rgb(cell.bg);

                let (fg_r, fg_g, fg_b) = if cell.bold {
                    (
                        (fg_r as u16 + 50).min(255) as u8,
                        (fg_g as u16 + 50).min(255) as u8,
                        (fg_b as u16 + 50).min(255) as u8,
                    )
                } else {
                    (fg_r, fg_g, fg_b)
                };

                let glyph_byte = if cell.ch.is_ascii() {
                    cell.ch as u8
                } else {
                    b'?'
                };
                draw_glyph(
                    &mut buffer,
                    buffer_width,
                    cell_x,
                    cell_y,
                    glyph_byte as char,
                    (fg_r, fg_g, fg_b),
                    (bg_r, bg_g, bg_b),
                    cell_width,
                    cell_height,
                );

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
                    layout,
                    layout
                        .terminal_top
                        .saturating_add(scaled_px(8, layout.ui_scale)),
                    &ui.status_chip,
                );
            }
            if !ui.ghost_suffix.is_empty() {
                self.render_ghost_text(&mut buffer, layout, y_off, screen, &ui.ghost_suffix);
            }
        }

        if layout.title_bar_h > 0 {
            if let Some(ui) = overlay {
                draw_title_bar(&mut buffer, layout, focus, ui);
            } else {
                draw_title_bar_minimal(&mut buffer, layout, focus);
            }
        }

        let (out_w, padded) = pad_rgba_right_edge(buffer, buffer_width, self.width, buffer_height);
        window.render_buffer(&padded, out_w, buffer_height)
    }

    fn render_status_chip(
        &self,
        buffer: &mut [u8],
        layout: &SplitLayout,
        chip_y: u32,
        text: &str,
    ) {
        if text.is_empty() {
            return;
        }
        let buffer_width = layout.buffer_width;
        let ui = layout.ui_scale;
        let chars = text.chars().count().min(72) as u32;
        let width = (chars + 3) * layout.ui_cell_width;
        let height = layout.ui_cell_height + scaled_px(6, ui);
        draw_panel_frame(
            buffer,
            buffer_width,
            scaled_px(6, ui),
            chip_y,
            width,
            height,
            UI_PANEL_BORDER,
            UI_ACCENT_SOFT,
        );
        draw_rect(
            buffer,
            buffer_width,
            scaled_px(10, ui),
            chip_y + scaled_px(4, ui),
            scaled_px(4, ui),
            scaled_px(4, ui),
            UI_ACCENT,
        );
        draw_text(
            buffer,
            buffer_width,
            scaled_px(18, ui),
            chip_y + scaled_px(3, ui),
            text,
            UI_TEXT,
            UI_ACCENT_SOFT,
            layout.ui_cell_width,
            layout.ui_cell_height,
        );
    }

    fn render_filesystem_panel(
        &self,
        buffer: &mut [u8],
        layout: &SplitLayout,
        ui: &UiOverlay,
    ) {
        let buffer_width = layout.buffer_width;
        let explorer_top = layout.explorer_top;
        let explorer_bottom = layout.separator_y;
        let ui_scale = layout.ui_scale;
        let ui_cell_h = layout.ui_cell_height;
        let ui_cell_w = layout.ui_cell_width;
        if explorer_bottom <= explorer_top {
            return;
        }
        let panel_h = explorer_bottom.saturating_sub(explorer_top);
        draw_vertical_gradient(
            buffer,
            buffer_width,
            0,
            explorer_top,
            buffer_width,
            panel_h,
            UI_SURFACE,
            UI_SURFACE_ALT,
        );

        let shell_x = scaled_px(6, ui_scale);
        let shell_y = explorer_top + scaled_px(4, ui_scale);
        let shell_w = buffer_width.saturating_sub(scaled_px(12, ui_scale));
        let shell_h = panel_h.saturating_sub(scaled_px(8, ui_scale));
        draw_panel_frame(
            buffer,
            buffer_width,
            shell_x,
            shell_y,
            shell_w,
            shell_h,
            UI_PANEL_BORDER,
            UI_SURFACE,
        );

        let panel_x = shell_x + scaled_px(6, ui_scale);
        let panel_width = shell_w.saturating_sub(scaled_px(12, ui_scale));
        let band = layout.list_band();
        let max_list = band.row_count;
        let mut y = shell_y + scaled_px(6, ui_scale);

        draw_rect(
            buffer,
            buffer_width,
            panel_x,
            y.saturating_sub(scaled_px(2, ui_scale)),
            panel_width,
            ui_cell_h + scaled_px(6, ui_scale),
            UI_SURFACE_ALT,
        );
        draw_text(
            buffer,
            buffer_width,
            panel_x + scaled_px(8, ui_scale),
            y + scaled_px(1, ui_scale),
            &ui.browser_title,
            UI_ACCENT,
            UI_SURFACE_ALT,
            ui_cell_w,
            ui_cell_h,
        );
        y = band.list_top;

        let list_bg_y = y.saturating_sub(scaled_px(2, ui_scale));
        let list_bg_h = max_list
            .saturating_mul(band.row_height)
            .saturating_add(scaled_px(4, ui_scale));
        draw_panel_frame(
            buffer,
            buffer_width,
            panel_x,
            list_bg_y,
            panel_width,
            list_bg_h,
            UI_PANEL_BORDER,
            UI_SURFACE_ALT,
        );

        for line in ui.browser_lines.iter().take(max_list as usize) {
            let selected = line.starts_with('>');
            let label = line.trim_start_matches('>').trim_start();
            if selected {
                draw_rect(
                    buffer,
                    buffer_width,
                    panel_x + 1,
                    y.saturating_sub(scaled_px(1, ui_scale)),
                    panel_width.saturating_sub(2),
                    band.row_height + scaled_px(2, ui_scale),
                    UI_ACCENT_SOFT,
                );
                draw_rect(
                    buffer,
                    buffer_width,
                    panel_x + 1,
                    y.saturating_sub(scaled_px(1, ui_scale)),
                    scaled_px(3, ui_scale),
                    band.row_height + scaled_px(2, ui_scale),
                    UI_ACCENT,
                );
            }
            draw_text(
                buffer,
                buffer_width,
                panel_x + scaled_px(10, ui_scale),
                y,
                label,
                if selected { UI_TEXT } else { UI_TEXT_MUTED },
                if selected {
                    UI_ACCENT_SOFT
                } else {
                    UI_SURFACE_ALT
                },
                ui_cell_w,
                ui_cell_h,
            );
            y += band.row_height;
        }

        let split_top = y + scaled_px(2, ui_scale);
        draw_rect(
            buffer,
            buffer_width,
            panel_x + 1,
            split_top,
            panel_width.saturating_sub(2),
            scaled_px(2, ui_scale).max(1),
            UI_TITLE_RULE,
        );
        y = split_top + scaled_px(6, ui_scale);
        let preview_h = explorer_bottom
            .saturating_sub(y)
            .saturating_sub(scaled_px(4, ui_scale));
        draw_panel_frame(
            buffer,
            buffer_width,
            panel_x,
            y,
            panel_width,
            preview_h,
            UI_PANEL_BORDER,
            UI_SURFACE,
        );
        draw_text(
            buffer,
            buffer_width,
            panel_x + scaled_px(8, ui_scale),
            y + scaled_px(3, ui_scale),
            &ui.preview_title,
            UI_ACCENT,
            UI_SURFACE,
            ui_cell_w,
            ui_cell_h,
        );
        y += ui_cell_h + scaled_px(8, ui_scale);
        let preview_rows = explorer_bottom
            .saturating_sub(y)
            .saturating_sub(ui_cell_h)
            / ui_cell_h.max(1);
        for line in ui
            .preview_lines
            .iter()
            .take(preview_rows.max(1).min(24) as usize)
        {
            if y >= explorer_bottom.saturating_sub(ui_cell_h) {
                break;
            }
            draw_text(
                buffer,
                buffer_width,
                panel_x + scaled_px(8, ui_scale),
                y,
                line,
                UI_TEXT_MUTED,
                UI_SURFACE,
                ui_cell_w,
                ui_cell_h,
            );
            y += ui_cell_h;
        }
    }

    fn render_ghost_text(
        &self,
        buffer: &mut [u8],
        layout: &SplitLayout,
        y_off: u32,
        screen: &ScreenBuffer,
        ghost_suffix: &str,
    ) {
        let x = (screen.cursor_col as u32) * layout.term_cell_width;
        let y = y_off + (screen.cursor_row as u32) * layout.term_cell_height;
        draw_text(
            buffer,
            layout.buffer_width,
            x,
            y,
            ghost_suffix,
            (122, 146, 182),
            TERM_BG_DEFAULT,
            layout.term_cell_width,
            layout.term_cell_height,
        );
    }
}

fn rect_hit(r: (u32, u32, u32, u32), x: u32, y: u32) -> bool {
    let (rx, ry, rw, rh) = r;
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

fn draw_title_bar_minimal(buffer: &mut [u8], layout: &SplitLayout, active: ContentView) {
    let buffer_width = layout.buffer_width;
    let bar_h = layout.title_bar_h;
    let ui = layout.ui_scale;
    draw_vertical_gradient(
        buffer,
        buffer_width,
        0,
        0,
        buffer_width,
        bar_h,
        UI_TITLE_BG,
        UI_SURFACE,
    );
    draw_rect(
        buffer,
        buffer_width,
        0,
        bar_h.saturating_sub(scaled_px(2, ui)),
        buffer_width,
        scaled_px(2, ui).max(1),
        UI_TITLE_RULE,
    );
    draw_title_bar_inner(buffer, layout, active, None);
}

fn draw_title_bar(buffer: &mut [u8], layout: &SplitLayout, active: ContentView, _ui: &UiOverlay) {
    draw_title_bar_minimal(buffer, layout, active);
}

fn draw_title_bar_inner(
    buffer: &mut [u8],
    layout: &SplitLayout,
    active: ContentView,
    _subtitle: Option<&str>,
) {
    let buffer_width = layout.buffer_width;
    let fw = layout.ui_cell_width;
    let ui_h = layout.ui_cell_height;
    let ui = layout.ui_scale;
    let regions = Renderer::title_bar_regions(layout);
    let label_y = layout.title_bar_h.saturating_sub(ui_h) / 2;

    draw_text(
        buffer,
        buffer_width,
        scaled_px(10, ui),
        label_y,
        "StratTerm",
        UI_TEXT,
        UI_TITLE_BG,
        fw,
        ui_h,
    );
    draw_text(
        buffer,
        buffer_width,
        scaled_px(10, ui) + (9 * fw),
        label_y,
        "F7 switch",
        UI_TEXT_MUTED,
        UI_TITLE_BG,
        fw,
        ui_h,
    );

    let (fx, fy, fwid, fhgt) = regions.files_tab;
    let files_active = active == ContentView::Filesystem;
    draw_panel_frame(
        buffer,
        buffer_width,
        fx,
        fy,
        fwid,
        fhgt,
        if files_active {
            UI_ACCENT
        } else {
            UI_PANEL_BORDER
        },
        if files_active {
            UI_ACCENT_SOFT
        } else {
            UI_SURFACE_ALT
        },
    );
    draw_text(
        buffer,
        buffer_width,
        fx + scaled_px(8, ui),
        label_y,
        "Files",
        if files_active { UI_TEXT } else { UI_TEXT_MUTED },
        if files_active {
            UI_ACCENT_SOFT
        } else {
            UI_SURFACE_ALT
        },
        fw,
        ui_h,
    );

    let (tx, ty, twid, thgt) = regions.terminal_tab;
    let term_active = active == ContentView::Terminal;
    draw_panel_frame(
        buffer,
        buffer_width,
        tx,
        ty,
        twid,
        thgt,
        if term_active {
            UI_ACCENT
        } else {
            UI_PANEL_BORDER
        },
        if term_active {
            UI_ACCENT_SOFT
        } else {
            UI_SURFACE_ALT
        },
    );
    draw_text(
        buffer,
        buffer_width,
        tx + scaled_px(8, ui),
        label_y,
        "Terminal",
        if term_active { UI_TEXT } else { UI_TEXT_MUTED },
        if term_active {
            UI_ACCENT_SOFT
        } else {
            UI_SURFACE_ALT
        },
        fw,
        ui_h,
    );

    let (cx, cy, cw, ch) = regions.close_btn;
    draw_panel_frame(
        buffer,
        buffer_width,
        cx,
        cy,
        cw,
        ch,
        UI_CLOSE_BG,
        (133, 56, 62),
    );
    draw_text(
        buffer,
        buffer_width,
        cx + cw / 2 - fw / 2 + 1,
        label_y,
        "X",
        UI_TEXT,
        (133, 56, 62),
        fw,
        ui_h,
    );
}

/// When the Wayland surface is wider than `cols * FONT_WIDTH`, extend each row with opaque black (BGR888 + A).
fn pad_rgba_right_edge(src: Vec<u8>, src_w: u32, dst_w: u32, height: u32) -> (u32, Vec<u8>) {
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

fn draw_panel_frame(
    buffer: &mut [u8],
    buffer_width: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    border: (u8, u8, u8),
    fill: (u8, u8, u8),
) {
    if width == 0 || height == 0 {
        return;
    }
    draw_rect(buffer, buffer_width, x, y, width, height, border);
    if width > 2 && height > 2 {
        draw_rect(
            buffer,
            buffer_width,
            x + 1,
            y + 1,
            width - 2,
            height - 2,
            fill,
        );
    }
}

fn draw_vertical_gradient(
    buffer: &mut [u8],
    buffer_width: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    top: (u8, u8, u8),
    bottom: (u8, u8, u8),
) {
    if width == 0 || height == 0 {
        return;
    }
    let den = height.saturating_sub(1).max(1) as u32;
    for row in 0..height {
        let r = lerp_u8(top.0, bottom.0, row, den);
        let g = lerp_u8(top.1, bottom.1, row, den);
        let b = lerp_u8(top.2, bottom.2, row, den);
        draw_rect(buffer, buffer_width, x, y + row, width, 1, (r, g, b));
    }
}

fn lerp_u8(a: u8, b: u8, num: u32, den: u32) -> u8 {
    if den == 0 {
        return a;
    }
    let a = a as i32;
    let b = b as i32;
    let v = a + ((b - a) * num as i32) / den as i32;
    v.clamp(0, 255) as u8
}

fn draw_text(
    buffer: &mut [u8],
    buffer_width: u32,
    x: u32,
    y: u32,
    text: &str,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
    glyph_w: u32,
    glyph_h: u32,
) {
    let glyph_w = glyph_w.max(1);
    let glyph_h = glyph_h.max(1);
    let mut cursor_x = x;
    for ch in text.chars().take(96) {
        draw_glyph(buffer, buffer_width, cursor_x, y, ch, fg, bg, glyph_w, glyph_h);
        cursor_x += glyph_w;
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
    glyph_w: u32,
    glyph_h: u32,
) {
    let stride = buffer_width as usize * 4;
    let max_height = buffer.len() / stride;
    let glyph_byte = if ch.is_ascii() { ch as u8 } else { b'?' };
    let glyph = FONT_DATA[glyph_byte as usize];

    let glyph_w = glyph_w.max(1);
    let glyph_h = glyph_h.max(1);
    for row in 0..glyph_h {
        let py = y + row;
        if py as usize >= max_height {
            continue;
        }
        let src_row = ((row as u64 * FONT_HEIGHT as u64) / glyph_h as u64)
            .min((FONT_HEIGHT - 1) as u64) as usize;
        let bits = glyph[src_row];
        let row_base = py as usize * stride;
        for col in 0..glyph_w {
            let px = x + col;
            if px >= buffer_width {
                continue;
            }
            let src_col = ((col as u64 * FONT_WIDTH as u64) / glyph_w as u64)
                .min((FONT_WIDTH - 1) as u64) as u32;
            // FONT_DATA rows store glyph bits in the upper byte (bits 15..8), LSB-left.
            let set = ((bits >> (8 + src_col)) & 1) == 1;
            let (r, g, b) = if set { fg } else { bg };
            let index = row_base + (px as usize * 4);
            buffer[index] = b;
            buffer[index + 1] = g;
            buffer[index + 2] = r;
            buffer[index + 3] = 255;
        }
    }
}
