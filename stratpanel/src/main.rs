use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use stratlayer::{
    Event, Interface, ShmBuffer, ShmPool, WaylandClient, WaylandSocket, WlCompositor, WlDisplay,
    WlRegistry, WlSeat, WlShm, WlShmPool, WlSurface, ZwlrLayerShellV1, ZwlrLayerSurfaceV1,
    ANCHOR_LEFT, ANCHOR_RIGHT, ANCHOR_TOP, LAYER_TOP,
};

mod clock;
mod config;
mod ipc;
mod textinput;

/// SHM pixels are authored upright at scale 1; align pending wl_surface state before commit.
#[inline]
fn surface_prepare_shm_commit(surface_id: u32, socket: &WaylandSocket) {
    let s = WlSurface::new(surface_id);
    s.set_buffer_transform(0, socket); // WL_OUTPUT_TRANSFORM_NORMAL
    s.set_buffer_scale(1, socket);
}

fn draw_text(
    buf: &mut [u8],
    stride: u32,
    panel_width: i32,
    panel_height: i32,
    x: i32,
    y: i32,
    text: &str,
    color: u32,
    scale: i32,
) {
    let scale = scale.max(1);
    const FONT: [(char, [u8; 7]); 95] = [
        ('0', [0x3E, 0x51, 0x49, 0x45, 0x3E, 0x00, 0x00]),
        ('1', [0x00, 0x42, 0x7F, 0x40, 0x00, 0x00, 0x00]),
        ('2', [0x42, 0x61, 0x51, 0x49, 0x46, 0x00, 0x00]),
        ('3', [0x21, 0x41, 0x45, 0x4B, 0x31, 0x00, 0x00]),
        ('4', [0x18, 0x14, 0x12, 0x7F, 0x10, 0x00, 0x00]),
        ('5', [0x27, 0x45, 0x45, 0x45, 0x39, 0x00, 0x00]),
        ('6', [0x3C, 0x4A, 0x49, 0x49, 0x30, 0x00, 0x00]),
        ('7', [0x01, 0x71, 0x09, 0x05, 0x03, 0x00, 0x00]),
        ('8', [0x36, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00]),
        ('9', [0x06, 0x49, 0x49, 0x29, 0x1E, 0x00, 0x00]),
        ('A', [0x7E, 0x09, 0x09, 0x09, 0x7E, 0x00, 0x00]),
        ('B', [0x7F, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00]),
        ('C', [0x3E, 0x41, 0x41, 0x41, 0x22, 0x00, 0x00]),
        ('D', [0x7F, 0x41, 0x41, 0x41, 0x3E, 0x00, 0x00]),
        ('E', [0x7F, 0x49, 0x49, 0x49, 0x41, 0x00, 0x00]),
        ('F', [0x7F, 0x09, 0x09, 0x09, 0x01, 0x00, 0x00]),
        ('G', [0x3E, 0x41, 0x49, 0x49, 0x3E, 0x00, 0x00]),
        ('H', [0x7F, 0x08, 0x08, 0x08, 0x7F, 0x00, 0x00]),
        ('I', [0x00, 0x41, 0x7F, 0x41, 0x00, 0x00, 0x00]),
        ('J', [0x1E, 0x20, 0x20, 0x20, 0x1F, 0x00, 0x00]),
        ('K', [0x7F, 0x08, 0x14, 0x22, 0x41, 0x00, 0x00]),
        ('L', [0x7F, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00]),
        ('M', [0x7F, 0x02, 0x0C, 0x02, 0x7F, 0x00, 0x00]),
        ('N', [0x7F, 0x04, 0x08, 0x10, 0x7F, 0x00, 0x00]),
        ('O', [0x7E, 0x41, 0x41, 0x41, 0x7E, 0x00, 0x00]),
        ('P', [0x7F, 0x09, 0x09, 0x09, 0x06, 0x00, 0x00]),
        ('Q', [0x7E, 0x41, 0x51, 0x21, 0x5E, 0x00, 0x00]),
        ('R', [0x7F, 0x09, 0x19, 0x29, 0x46, 0x00, 0x00]),
        ('S', [0x46, 0x49, 0x49, 0x49, 0x31, 0x00, 0x00]),
        ('T', [0x01, 0x01, 0x7F, 0x01, 0x01, 0x00, 0x00]),
        ('U', [0x7F, 0x40, 0x40, 0x40, 0x3F, 0x00, 0x00]),
        ('V', [0x1F, 0x20, 0x40, 0x20, 0x1F, 0x00, 0x00]),
        ('W', [0x3F, 0x40, 0x38, 0x40, 0x3F, 0x00, 0x00]),
        ('X', [0x63, 0x14, 0x08, 0x14, 0x63, 0x00, 0x00]),
        ('Y', [0x03, 0x04, 0x78, 0x04, 0x03, 0x00, 0x00]),
        ('Z', [0x61, 0x51, 0x49, 0x45, 0x43, 0x00, 0x00]),
        ('a', [0x20, 0x54, 0x54, 0x54, 0x78, 0x00, 0x00]),
        ('b', [0x7F, 0x48, 0x44, 0x44, 0x38, 0x00, 0x00]),
        ('c', [0x38, 0x44, 0x44, 0x44, 0x20, 0x00, 0x00]),
        ('d', [0x38, 0x44, 0x44, 0x48, 0x7F, 0x00, 0x00]),
        ('e', [0x38, 0x54, 0x54, 0x54, 0x18, 0x00, 0x00]),
        ('f', [0x08, 0x7E, 0x09, 0x01, 0x02, 0x00, 0x00]),
        ('g', [0x08, 0x14, 0x54, 0x54, 0x3C, 0x00, 0x00]),
        ('h', [0x7F, 0x08, 0x04, 0x04, 0x78, 0x00, 0x00]),
        ('i', [0x00, 0x44, 0x7D, 0x40, 0x00, 0x00, 0x00]),
        ('j', [0x20, 0x40, 0x44, 0x3D, 0x00, 0x00, 0x00]),
        ('k', [0x7F, 0x10, 0x28, 0x44, 0x00, 0x00, 0x00]),
        ('l', [0x00, 0x41, 0x7F, 0x40, 0x00, 0x00, 0x00]),
        ('m', [0x7C, 0x04, 0x78, 0x04, 0x78, 0x00, 0x00]),
        ('n', [0x7C, 0x08, 0x04, 0x04, 0x78, 0x00, 0x00]),
        ('o', [0x38, 0x44, 0x44, 0x44, 0x38, 0x00, 0x00]),
        ('p', [0x7C, 0x14, 0x14, 0x14, 0x08, 0x00, 0x00]),
        ('q', [0x08, 0x14, 0x14, 0x18, 0x7C, 0x00, 0x00]),
        ('r', [0x7C, 0x08, 0x04, 0x04, 0x08, 0x00, 0x00]),
        ('s', [0x48, 0x54, 0x54, 0x54, 0x24, 0x00, 0x00]),
        ('t', [0x04, 0x7F, 0x44, 0x40, 0x20, 0x00, 0x00]),
        ('u', [0x3C, 0x40, 0x40, 0x20, 0x7C, 0x00, 0x00]),
        ('v', [0x1C, 0x20, 0x40, 0x20, 0x1C, 0x00, 0x00]),
        ('w', [0x3C, 0x40, 0x30, 0x40, 0x3C, 0x00, 0x00]),
        ('x', [0x44, 0x28, 0x10, 0x28, 0x44, 0x00, 0x00]),
        ('y', [0x0C, 0x50, 0x50, 0x50, 0x3C, 0x00, 0x00]),
        ('z', [0x44, 0x64, 0x54, 0x4C, 0x44, 0x00, 0x00]),
        (' ', [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ('!', [0x00, 0x5F, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ('@', [0x3C, 0x4A, 0x4A, 0x3C, 0x00, 0x00, 0x00]),
        ('#', [0x14, 0x7F, 0x14, 0x7F, 0x14, 0x00, 0x00]),
        ('$', [0x24, 0x2A, 0x7F, 0x2A, 0x12, 0x00, 0x00]),
        ('%', [0x62, 0x64, 0x08, 0x13, 0x23, 0x00, 0x00]),
        ('^', [0x04, 0x02, 0x01, 0x02, 0x04, 0x00, 0x00]),
        ('&', [0x36, 0x49, 0x55, 0x22, 0x50, 0x00, 0x00]),
        ('*', [0x44, 0x28, 0x7F, 0x28, 0x44, 0x00, 0x00]),
        ('(', [0x0E, 0x11, 0x11, 0x11, 0x0E, 0x00, 0x00]),
        (')', [0x70, 0x88, 0x88, 0x88, 0x70, 0x00, 0x00]),
        ('-', [0x00, 0x08, 0x7F, 0x08, 0x00, 0x00, 0x00]),
        ('_', [0x00, 0x00, 0x00, 0x00, 0x7F, 0x00, 0x00]),
        ('+', [0x00, 0x08, 0x2A, 0x08, 0x00, 0x00, 0x00]),
        ('=', [0x00, 0x14, 0x14, 0x14, 0x00, 0x00, 0x00]),
        ('[', [0x7F, 0x41, 0x41, 0x00, 0x00, 0x00, 0x00]),
        (']', [0x41, 0x41, 0x7F, 0x00, 0x00, 0x00, 0x00]),
        ('{', [0x14, 0x12, 0x7F, 0x12, 0x14, 0x00, 0x00]),
        ('}', [0x14, 0x48, 0x7F, 0x48, 0x14, 0x00, 0x00]),
        (':', [0x00, 0x36, 0x36, 0x00, 0x00, 0x00, 0x00]),
        (';', [0x00, 0x56, 0x36, 0x00, 0x00, 0x00, 0x00]),
        ('\'', [0x00, 0x06, 0x09, 0x00, 0x00, 0x00, 0x00]),
        ('"', [0x06, 0x09, 0x06, 0x09, 0x00, 0x00, 0x00]),
        ('`', [0x04, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ('~', [0x08, 0x34, 0x28, 0x34, 0x08, 0x00, 0x00]),
        ('\\', [0x40, 0x20, 0x10, 0x08, 0x04, 0x00, 0x00]),
        ('|', [0x00, 0x7F, 0x00, 0x7F, 0x00, 0x00, 0x00]),
        (',', [0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00]),
        ('.', [0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00]),
        ('/', [0x04, 0x08, 0x10, 0x20, 0x40, 0x00, 0x00]),
        ('<', [0x08, 0x14, 0x22, 0x41, 0x00, 0x00, 0x00]),
        ('>', [0x41, 0x22, 0x14, 0x08, 0x00, 0x00, 0x00]),
        ('?', [0x02, 0x01, 0x51, 0x09, 0x06, 0x00, 0x00]),
    ];

    let bytes = color.to_le_bytes();
    let mut cursor_x = x;

    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += 6 * scale;
            continue;
        }

        let glyph = FONT.iter().find(|(c, _)| *c == ch);
        if let Some((_, cols)) = glyph {
            // FONT bytes are column-major: first 5 bytes are columns, LSB is top pixel.
            for col_idx in 0..5 {
                let col = cols[col_idx];
                for row_idx in 0..7 {
                    if (col >> row_idx) & 1 == 1 {
                        let px0 = cursor_x + col_idx as i32 * scale;
                        let py0 = y + row_idx as i32 * scale;
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = px0 + sx;
                                let py = py0 + sy;
                                if px >= 0 && px < panel_width && py >= 0 && py < panel_height {
                                    let offset = (py as u32 * stride + px as u32 * 4) as usize;
                                    if offset + 4 <= buf.len() {
                                        buf[offset..offset + 4].copy_from_slice(&bytes);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        cursor_x += 6 * scale;
    }
}

fn fill_rect(
    buf: &mut [u8],
    stride: u32,
    panel_width: i32,
    panel_height: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
) {
    let bytes = color.to_le_bytes();
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(panel_width);
    let y1 = (y + h).min(panel_height);

    for py in y0..y1 {
        for px in x0..x1 {
            let offset = (py as u32 * stride + px as u32 * 4) as usize;
            if offset + 4 <= buf.len() {
                buf[offset..offset + 4].copy_from_slice(&bytes);
            }
        }
    }
}

const PEEK_H: u32 = 4;
const BTN_LEFT: u32 = 0x110;
/// `wl_pointer.axis` value 0 — vertical scroll.
const AXIS_SCROLL_VERTICAL: u32 = 0;

const PANEL_BG_RGB: u32 = 0x0E1728;
const PANEL_SURFACE_RGB: u32 = 0x162338;
const PANEL_SURFACE_ALT_RGB: u32 = 0x1D2C45;
const PANEL_BORDER_RGB: u32 = 0x2E4569;
const PANEL_ACCENT_RGB: u32 = 0x73BCFF;
const PANEL_ACCENT_SOFT_RGB: u32 = 0x2D4870;
const PANEL_TEXT_MUTED: u32 = 0xFF9FB2D0;
const PANEL_TEXT_MAIN: u32 = 0xFFF1F6FF;

fn quantize_font_scale(scale: f32) -> i32 {
    if !scale.is_finite() {
        return 1;
    }
    scale.clamp(1.0, 4.0).round() as i32
}

#[inline]
fn panel_rgba(opacity: f64, rgb: u32) -> u32 {
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    (alpha << 24) | (rgb & 0x00FF_FFFF)
}

/// Workspace buttons shown in the panel (respects switcher enable, label mode, max visible).
fn visible_workspaces(
    raw: &[(u32, String, bool)],
    panel_cfg: &config::PanelConfig,
) -> Vec<(u32, String, bool)> {
    if !panel_cfg.workspace.enabled {
        return Vec::new();
    }
    let cap = if panel_cfg.workspace.max_visible > 0 {
        panel_cfg.workspace.max_visible as usize
    } else {
        raw.len()
    };
    let slice: Vec<_> = raw.iter().take(cap).cloned().collect();
    if panel_cfg.workspace.show_labels {
        slice
    } else {
        slice
            .into_iter()
            .enumerate()
            .map(|(i, (id, _name, focused))| (id, format!("{}", i + 1), focused))
            .collect()
    }
}

/// Geometry for the visible top bar (shared by draw + pointer handlers).
struct TopBarLayout {
    glyph_advance: i32,
    glyph_h: i32,
    input_x: i32,
    input_w: i32,
    tray_slot: i32,
    clock_x: i32,
    tray_x: i32,
    pinned_x: i32,
    pinned_max_w: i32,
    pin_cell: i32,
    pin_gap: i32,
    btn_w: i32,
    ws_start_x: i32,
}

impl TopBarLayout {
    fn compute(
        panel_width: i32,
        clock_text: &str,
        ws_count: usize,
        config: &config::PanelConfig,
        scale: i32,
    ) -> Self {
        let scale = scale.max(1);
        let glyph_advance = 6 * scale;
        let glyph_h = 7 * scale;
        let input_x = 12i32 * scale;
        let input_w = (panel_width / 4).clamp(180 * scale, 360 * scale);
        let pinned_left = input_x + input_w + 10 * scale;
        let clock_tw = clock_text.len() as i32 * glyph_advance;
        let tray_slot = 26i32 * scale;
        let tray_n = [
            config.tray.show_network,
            config.tray.show_volume,
            config.tray.show_updates,
            config.tray.show_battery,
        ]
        .iter()
        .filter(|&&x| x)
        .count() as i32;
        let tray_w = if tray_n > 0 {
            tray_n * tray_slot + 6 * scale
        } else {
            0i32
        };
        let clock_x = panel_width - clock_tw - 14 * scale;
        let tray_x = clock_x - 10 * scale - tray_w;
        let pinned_x = pinned_left;
        let pinned_max_w = (tray_x - 12 * scale - pinned_x).max(0);
        let pin_cell = 34i32 * scale;
        let pin_gap = 6i32 * scale;
        let btn_w = 46i32 * scale;
        let ws_total = if ws_count == 0 {
            0
        } else {
            ws_count as i32 * (btn_w + 6 * scale) - 6 * scale
        };
        let ws_start_x = (panel_width - ws_total) / 2;
        Self {
            glyph_advance,
            glyph_h,
            input_x,
            input_w,
            tray_slot,
            clock_x,
            tray_x,
            pinned_x,
            pinned_max_w,
            pin_cell,
            pin_gap,
            btn_w,
            ws_start_x,
        }
    }

    /// X and width of the volume tray cell (`[tray] show_volume`), matching draw order: N, V, U, B.
    fn volume_cell_bounds(&self, cfg: &config::PanelConfig) -> Option<(i32, i32)> {
        if !cfg.tray.show_volume {
            return None;
        }
        let mut tx = self.tray_x;
        if cfg.tray.show_network {
            tx += self.tray_slot;
        }
        Some((tx, self.tray_slot - 2))
    }
}

fn pointer_in_tray_volume_cell(
    px: i32,
    py: i32,
    vis_h: i32,
    layout: &TopBarLayout,
    cfg: &config::PanelConfig,
    scale: i32,
) -> bool {
    let pad = 2 * scale.max(1);
    if py < pad || py >= vis_h - pad {
        return false;
    }
    let Some((vx, w)) = layout.volume_cell_bounds(cfg) else {
        return false;
    };
    px >= vx && px < vx + w
}

/// Kernel `operstate` for the tray network icon (any non-`lo` iface may satisfy “connected”).
#[derive(Clone, Copy, PartialEq, Eq)]
enum NetworkTrayState {
    Connected,
    Disconnected,
    Unknown,
}

fn tray_network_state() -> NetworkTrayState {
    let mut names: Vec<String> = match fs::read_dir("/sys/class/net") {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n != "lo")
            .collect(),
        Err(_) => return NetworkTrayState::Unknown,
    };
    if names.is_empty() {
        return NetworkTrayState::Unknown;
    }
    names.sort();
    let mut any_up = false;
    let mut any_oper = false;
    for n in &names {
        let path = format!("/sys/class/net/{}/operstate", n);
        if let Ok(s) = fs::read_to_string(&path) {
            any_oper = true;
            if s.trim() == "up" {
                any_up = true;
            }
        }
    }
    if any_up {
        NetworkTrayState::Connected
    } else if any_oper {
        NetworkTrayState::Disconnected
    } else {
        NetworkTrayState::Unknown
    }
}

/// Signal-bar style icon (14×10), centered in the tray cell.
fn draw_network_icon(
    buf: &mut [u8],
    stride: u32,
    panel_width: i32,
    panel_height: i32,
    cell_left: i32,
    cell_w: i32,
    vis_h: i32,
    state: NetworkTrayState,
    live: u32,
    stub: u32,
    scale: i32,
) {
    let scale = scale.max(1);
    let icon_w: i32 = 14 * scale;
    let icon_h: i32 = 10 * scale;
    let content_h = vis_h - 4;
    let ox = cell_left + (cell_w - icon_w) / 2;
    let oy = 2 + (content_h - icon_h) / 2;
    match state {
        NetworkTrayState::Unknown => {
            let tx = ox + (icon_w - 6 * scale) / 2;
            let ty = oy + (icon_h - 7 * scale) / 2;
            draw_text(
                buf,
                stride,
                panel_width,
                panel_height,
                tx,
                ty,
                "?",
                stub,
                scale,
            );
        }
        NetworkTrayState::Connected => {
            let bar_w = 2i32 * scale;
            let gap = 2i32 * scale;
            let heights = [3i32, 5, 7, 9].map(|v| v * scale);
            let base = oy + icon_h;
            for (i, h) in heights.iter().enumerate() {
                let bx = ox + i as i32 * (bar_w + gap);
                let by = base - *h;
                fill_rect(
                    buf,
                    stride,
                    panel_width,
                    panel_height,
                    bx,
                    by,
                    bar_w,
                    *h,
                    live,
                );
            }
        }
        NetworkTrayState::Disconnected => {
            let bar_w = 2i32 * scale;
            let gap = 2i32 * scale;
            let h = 3i32 * scale;
            let base = oy + icon_h;
            for i in 0..4i32 {
                let bx = ox + i * (bar_w + gap);
                let by = base - h;
                fill_rect(
                    buf,
                    stride,
                    panel_width,
                    panel_height,
                    bx,
                    by,
                    bar_w,
                    h,
                    stub,
                );
            }
        }
    }
}

fn tray_battery_label() -> String {
    if let Ok(rd) = fs::read_dir("/sys/class/power_supply") {
        for e in rd.flatten() {
            let name = e.file_name();
            let n = name.to_string_lossy();
            if !n.starts_with("BAT") {
                continue;
            }
            let path = format!("/sys/class/power_supply/{}/status", n);
            if let Ok(s) = fs::read_to_string(&path) {
                let t = s.trim();
                let tag = if t == "Charging" || t == "Full" {
                    "+"
                } else if t == "Discharging" {
                    "-"
                } else {
                    "~"
                };
                return format!("B{tag}");
            }
        }
    }
    "B~".to_string()
}

/// ALSA **Master** via `amixer` (alsa-utils). Label: `Vm` = muted, `V00`–`V99` = unmuted %, `V~` = no backend.
fn tray_volume_label() -> String {
    let Ok(output) = Command::new("amixer")
        .args(["-M", "get", "Master"])
        .output()
    else {
        return "V~".to_string();
    };
    if !output.status.success() {
        return "V~".to_string();
    }
    let s = String::from_utf8_lossy(&output.stdout);
    format_volume_from_amixer(&s)
}

fn format_volume_from_amixer(s: &str) -> String {
    let (muted, pct) = parse_amixer_master(s);
    if muted {
        return "Vm".to_string();
    }
    if let Some(p) = pct {
        return format!("V{:02}", p.min(99));
    }
    "V~".to_string()
}

fn parse_amixer_master(s: &str) -> (bool, Option<u8>) {
    let mut muted = false;
    let mut pct: Option<u8> = None;
    for line in s.lines() {
        if !line.contains("Playback") && !line.contains("Mono:") {
            continue;
        }
        for tok in line.split_whitespace() {
            if tok == "[off]" {
                muted = true;
            }
            if tok.starts_with('[') && tok.ends_with(']') && tok.contains('%') {
                let inner = &tok[1..tok.len() - 1];
                if let Some(num) = inner.strip_suffix('%') {
                    if let Ok(p) = num.parse::<u8>() {
                        pct = Some(p);
                    }
                }
            }
        }
    }
    (muted, pct)
}

fn volume_toggle_mute() {
    let _ = Command::new("amixer")
        .args(["-q", "-M", "set", "Master", "toggle"])
        .status();
}

/// Relative volume change using `amixer` percent steps (`3%+` / `3%-`).
fn volume_nudge_by_percent(delta: i32) {
    let d = delta.clamp(-10, 10);
    if d == 0 {
        return;
    }
    let arg = if d > 0 {
        format!("{}%+", d)
    } else {
        format!("{}%-", -d)
    };
    let _ = Command::new("amixer")
        .args(["-q", "-M", "set", "Master", &arg])
        .status();
}

fn launcher_label(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| {
            let t = s.trim();
            if t.len() <= 3 {
                t.to_uppercase()
            } else {
                t[..3].to_uppercase()
            }
        })
        .unwrap_or_else(|| "?".to_string())
}

fn launch_detached(path: &str) {
    if !path.starts_with('/') || path.contains('\0') {
        eprintln!("stratpanel: invalid launcher path (must be absolute)");
        return;
    }
    if !Path::new(path).exists() {
        eprintln!("stratpanel: launcher not found: {}", path);
        return;
    }
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run".into());
    if let Err(e) = Command::new(path)
        .env("WAYLAND_DISPLAY", wayland)
        .env("XDG_RUNTIME_DIR", runtime)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        eprintln!("stratpanel: spawn {} failed: {}", path, e);
    }
}

fn launch_from_input(input: &str) {
    let cmd = input.trim();
    if cmd.is_empty() {
        return;
    }
    if cmd.contains('\0') {
        eprintln!("stratpanel: launcher command contains NUL byte");
        return;
    }
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run".into());
    let path = std::env::var("PATH")
        .unwrap_or_else(|_| "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into());
    if let Err(e) = Command::new("/bin/sh")
        .arg("-lc")
        .arg(cmd)
        .env("WAYLAND_DISPLAY", wayland)
        .env("XDG_RUNTIME_DIR", runtime)
        .env("PATH", path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        eprintln!("stratpanel: launcher command failed `{}`: {}", cmd, e);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Connect to Wayland
    let mut client = WaylandClient::new()?;

    // Setup: send get_registry using allocate() so next_id advances past it
    let registry_id = client.registry().allocate();
    client
        .registry()
        .set_interface(registry_id, Interface::WlRegistry);
    WlDisplay::new(1).get_registry(registry_id, client.socket());

    // Step 2: Roundtrip to collect RegistryGlobal events
    let globals = client.roundtrip()?;

    // Load configuration
    let config = config::PanelConfig::load();
    let panel_font_scale = quantize_font_scale(config.panel.font_scale);
    let panel_size_px = (config.panel.size as i32).max(20 * panel_font_scale);

    // Connect to stratvm IPC
    let mut ipc = ipc::IpcClient::connect();
    ipc.set_panel_autohide(config.panel.autohide);

    let mut compositor_name: Option<u32> = None;
    let mut shm_name: Option<u32> = None;
    let mut layer_shell_name: Option<u32> = None;
    let mut seat_name: Option<u32> = None;

    for event in &globals {
        if let Event::RegistryGlobal {
            name, interface, ..
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => compositor_name = Some(*name),
                "wl_shm" => shm_name = Some(*name),
                "zwlr_layer_shell_v1" => layer_shell_name = Some(*name),
                "wl_seat" => seat_name = Some(*name),
                _ => {}
            }
        }
    }

    let compositor_name = compositor_name.ok_or("wl_compositor not found")?;
    let shm_name = shm_name.ok_or("wl_shm not found")?;
    let layer_shell_name = layer_shell_name.ok_or("zwlr_layer_shell_v1 not found")?;

    // Bind wl_compositor
    let compositor_id = client.registry().allocate();
    client
        .registry()
        .set_interface(compositor_id, Interface::WlCompositor);
    /* v5: wl_surface.offset; v4: damage_buffer — use buffer-space damage + explicit transform/scale. */
    WlRegistry::new(registry_id).bind(
        compositor_name,
        "wl_compositor",
        5,
        compositor_id,
        client.socket(),
    );

    // Bind wl_shm
    let shm_id = client.registry().allocate();
    client.registry().set_interface(shm_id, Interface::WlShm);
    WlRegistry::new(registry_id).bind(shm_name, "wl_shm", 1, shm_id, client.socket());

    // Bind zwlr_layer_shell_v1
    let layer_shell_id = client.registry().allocate();
    // No Interface variant for ZwlrLayerShellV1 in the enum; use Unknown
    WlRegistry::new(registry_id).bind(
        layer_shell_name,
        "zwlr_layer_shell_v1",
        4,
        layer_shell_id,
        client.socket(),
    );

    // Bind wl_seat
    let seat_id = client.registry().allocate();
    client.registry().set_interface(seat_id, Interface::WlSeat);
    let seat_name = seat_name.ok_or("wl_seat not found")?;
    WlRegistry::new(registry_id).bind(seat_name, "wl_seat", 7, seat_id, client.socket());

    // Get pointer from seat
    let pointer_id = client.registry().allocate();
    client
        .registry()
        .set_interface(pointer_id, Interface::WlPointer);
    WlSeat::new(seat_id).get_pointer(pointer_id, client.socket());

    // Get keyboard from seat
    let keyboard_id = client.registry().allocate();
    client
        .registry()
        .set_interface(keyboard_id, Interface::WlKeyboard);
    WlSeat::new(seat_id).get_keyboard(keyboard_id, client.socket());

    // Step 3: Create wl_surface
    let surface_id = client.registry().allocate();
    client
        .registry()
        .set_interface(surface_id, Interface::WlSurface);
    WlCompositor::new(compositor_id).create_surface(surface_id, client.socket());

    // Step 4: Create layer surface
    let layer_surface_id = client.registry().allocate();
    client.register_layer_surface(layer_surface_id);
    ZwlrLayerShellV1::new(layer_shell_id).get_layer_surface(
        layer_surface_id,
        surface_id,
        0,
        LAYER_TOP,
        "stratpanel",
        client.socket(),
    );

    // Step 5: Configure layer surface
    let ls = ZwlrLayerSurfaceV1::new(layer_surface_id);
    ls.set_size(0, panel_size_px as u32, client.socket());
    ls.set_anchor(ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT, client.socket());
    ls.set_exclusive_zone(panel_size_px, client.socket());
    ls.set_keyboard_interactivity(1, client.socket());
    surface_prepare_shm_commit(surface_id, client.socket());
    WlSurface::new(surface_id).commit(client.socket());

    // Step 6: Wait for LayerSurfaceConfigure
    let (confirmed_width, confirmed_height);
    'configure: loop {
        for event in client.poll()? {
            if let Event::LayerSurfaceConfigure {
                serial,
                width,
                height,
                ..
            } = event
            {
                ls.ack_configure(serial, client.socket());
                confirmed_width = width;
                confirmed_height = height;
                break 'configure;
            }
        }
    }

    // Step 7: Allocate SHM buffer (full design height; compositor clips when autohide uses PEEK_H)
    let panel_width = if confirmed_width == 0 {
        1920
    } else {
        confirmed_width
    };
    let buffer_draw_h = panel_size_px;
    let stride = panel_width * 4;
    let size = (stride * buffer_draw_h as u32) as usize;

    let pool = ShmPool::create(size)?;
    let shm_fd = pool.fd();

    // Fill with configurable-opacity panel background (ARGB8888)
    let mut shm_buffer = ShmBuffer::new(pool, 0, panel_width, buffer_draw_h as u32, stride);
    {
        let data = shm_buffer.data_mut();
        let color = panel_rgba(config.panel.opacity, PANEL_BG_RGB);
        let bytes = color.to_le_bytes();
        for chunk in data.chunks_exact_mut(4) {
            chunk[0] = bytes[0];
            chunk[1] = bytes[1];
            chunk[2] = bytes[2];
            chunk[3] = bytes[3];
        }
    }

    // Create wl_shm_pool
    let pool_id = client.registry().allocate();
    client
        .registry()
        .set_interface(pool_id, Interface::WlShmPool);
    WlShm::new(shm_id).create_pool(pool_id, shm_fd, size as i32, client.socket());

    // Create wl_buffer from pool
    let buffer_id = client.registry().allocate();
    client
        .registry()
        .set_interface(buffer_id, Interface::WlBuffer);
    WlShmPool::new(pool_id).create_buffer(
        buffer_id,
        0,
        panel_width as i32,
        buffer_draw_h,
        stride as i32,
        0, // WL_SHM_FORMAT_ARGB8888 = 0
        client.socket(),
    );

    // Step 8: Attach buffer and commit
    surface_prepare_shm_commit(surface_id, client.socket());
    WlSurface::new(surface_id).attach(buffer_id, 0, 0, client.socket());
    WlSurface::new(surface_id).damage_buffer(
        0,
        0,
        panel_width as i32,
        buffer_draw_h,
        client.socket(),
    );
    WlSurface::new(surface_id).commit(client.socket());

    let mut layer_surface_height = if confirmed_height == 0 {
        buffer_draw_h
    } else {
        confirmed_height as i32
    };

    if config.panel.autohide {
        ls.set_size(0, PEEK_H, client.socket());
        ls.set_exclusive_zone(0, client.socket());
        surface_prepare_shm_commit(surface_id, client.socket());
        WlSurface::new(surface_id).commit(client.socket());
        'peek: loop {
            for event in client.poll()? {
                if let Event::LayerSurfaceConfigure { serial, height, .. } = event {
                    ls.ack_configure(serial, client.socket());
                    if height > 0 {
                        layer_surface_height = height as i32;
                    }
                    break 'peek;
                }
            }
        }
    }

    // Initialize clock + workspaces (avoid empty clock until first 1s tick).
    let mut clock = clock::Clock::new();
    clock.tick(&config.clock.format, config.clock.show_date);
    let mut last_clock_text = clock.text().to_string();

    let mut workspaces_raw: Vec<(u32, String, bool)> = if config.workspace.enabled {
        ipc.get_workspaces()
    } else {
        vec![]
    };
    let mut workspaces = visible_workspaces(&workspaces_raw, &config);
    let mut tray_net_state = tray_network_state();
    let mut tray_vol_lbl = tray_volume_label();
    let mut tray_bat_lbl = tray_battery_label();
    let mut last_clock_tick = Instant::now();
    let mut last_workspace_fetch = Instant::now();

    // Initialize pointer state
    let mut pointer_x: f64 = 0.0;
    let mut pointer_y: f64 = 0.0;

    // Initialize text input
    let mut text_input = textinput::TextInput::new();
    let mut keyboard_focused = false;

    // Cursor blink state
    let mut cursor_visible = true;
    let mut last_cursor_blink = Instant::now();

    // Pinned strip horizontal scroll (pixels)
    let mut pinned_scroll: i32 = 0;

    // Autohide: collapse after pointer leaves (debounced)
    let mut autohide_collapse_at: Option<Instant> = None;

    // Track last rendered state to avoid unnecessary commits
    let mut needs_commit = true; // Initial render
    let mut last_configure_serial: u32 = 0;

    // Step 9: Main event loop
    loop {
        if last_clock_tick.elapsed() >= Duration::from_secs(1) {
            tray_net_state = tray_network_state();
            tray_vol_lbl = tray_volume_label();
            tray_bat_lbl = tray_battery_label();
            clock.tick(&config.clock.format, config.clock.show_date);
            let clock_text = clock.text();
            if clock_text != last_clock_text {
                last_clock_text = clock_text.to_string();
            }
            last_clock_tick = Instant::now();
            needs_commit = true;
        }
        let ws_secs = config.workspace.poll_interval_secs.max(1);
        if config.workspace.enabled
            && last_workspace_fetch.elapsed() >= Duration::from_secs(ws_secs)
        {
            workspaces_raw = ipc.get_workspaces();
            workspaces = visible_workspaces(&workspaces_raw, &config);
            last_workspace_fetch = Instant::now();
            needs_commit = true;
        }

        // Blink cursor
        if last_cursor_blink.elapsed() >= Duration::from_millis(500) {
            cursor_visible = !cursor_visible;
            last_cursor_blink = Instant::now();
            if keyboard_focused {
                needs_commit = true; // Cursor visibility changed
            }
        }

        if let Some(t) = autohide_collapse_at {
            if Instant::now() >= t {
                autohide_collapse_at = None;
                if config.panel.autohide && layer_surface_height > PEEK_H as i32 {
                    ls.set_size(0, PEEK_H, client.socket());
                    ls.set_exclusive_zone(0, client.socket());
                    surface_prepare_shm_commit(surface_id, client.socket());
                    WlSurface::new(surface_id).commit(client.socket());
                    layer_surface_height = PEEK_H as i32;
                    needs_commit = true;
                }
            }
        }

        // Only render and commit if something changed
        if needs_commit {
            let ph = buffer_draw_h;
            let vis_h = layer_surface_height.min(ph);
            // Clear full drawable buffer
            {
                let data = shm_buffer.data_mut();
                let color = panel_rgba(config.panel.opacity, PANEL_BG_RGB);
                let bytes = color.to_le_bytes();
                for chunk in data.chunks_exact_mut(4) {
                    chunk[0] = bytes[0];
                    chunk[1] = bytes[1];
                    chunk[2] = bytes[2];
                    chunk[3] = bytes[3];
                }
            }

            if vis_h <= PEEK_H as i32 {
                let data = shm_buffer.data_mut();
                let accent = panel_rgba(config.panel.opacity, PANEL_ACCENT_SOFT_RGB);
                fill_rect(
                    data,
                    stride,
                    panel_width as i32,
                    ph,
                    0,
                    0,
                    panel_width as i32,
                    vis_h,
                    accent,
                );
            } else {
                let layout = TopBarLayout::compute(
                    panel_width as i32,
                    &last_clock_text,
                    workspaces.len(),
                    &config,
                    panel_font_scale,
                );
                let input_y = 2i32 * panel_font_scale;
                let input_h = (vis_h - 4 * panel_font_scale).max(layout.glyph_h + 2);
                let pointer_px = pointer_x as i32;
                let pointer_py = pointer_y as i32;

                {
                    let data = shm_buffer.data_mut();
                    fill_rect(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        0,
                        0,
                        panel_width as i32,
                        vis_h,
                        panel_rgba(config.panel.opacity, PANEL_SURFACE_RGB),
                    );
                    fill_rect(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        0,
                        0,
                        panel_width as i32,
                        1,
                        panel_rgba(config.panel.opacity, PANEL_BORDER_RGB),
                    );
                    fill_rect(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        0,
                        vis_h - 1,
                        panel_width as i32,
                        1,
                        panel_rgba(config.panel.opacity, PANEL_BORDER_RGB),
                    );
                }

                {
                    let data = shm_buffer.data_mut();
                    let input_border = panel_rgba(config.panel.opacity, PANEL_BORDER_RGB);
                    let input_bg = panel_rgba(
                        config.panel.opacity,
                        if keyboard_focused {
                            PANEL_SURFACE_ALT_RGB
                        } else {
                            PANEL_BG_RGB
                        },
                    );
                    fill_rect(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        layout.input_x,
                        input_y,
                        layout.input_w,
                        input_h,
                        input_border,
                    );
                    fill_rect(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        layout.input_x + 1,
                        input_y + 1,
                        layout.input_w - 2,
                        input_h - 2,
                        input_bg,
                    );

                    let input_text_left = layout.input_x + 6 * panel_font_scale;
                    let input_inner_w = (layout.input_w - 12 * panel_font_scale).max(6);
                    let visible_chars =
                        (input_inner_w / layout.glyph_advance.max(1)).max(1) as usize;
                    text_input.ensure_cursor_visible(visible_chars);
                    let display_text = text_input.display_text(visible_chars);
                    let shown = if display_text.is_empty() {
                        "Launcher / command"
                    } else {
                        &display_text
                    };
                    let text_y = input_y + (input_h - layout.glyph_h) / 2;
                    draw_text(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        input_text_left,
                        text_y,
                        shown,
                        if display_text.is_empty() {
                            PANEL_TEXT_MUTED
                        } else {
                            PANEL_TEXT_MAIN
                        },
                        panel_font_scale,
                    );

                    if keyboard_focused && cursor_visible {
                        let cursor_x = input_text_left
                            + text_input.cursor_pixel_offset(layout.glyph_advance);
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            cursor_x,
                            text_y - panel_font_scale,
                            panel_font_scale,
                            layout.glyph_h + 2 * panel_font_scale,
                            0xFF73BCFF,
                        );
                    }
                }

                let pins_content = if config.pinned.apps.is_empty() {
                    0i32
                } else {
                    config.pinned.apps.len() as i32 * (layout.pin_cell + layout.pin_gap)
                        - layout.pin_gap
                };
                let pinned_max_w = layout.pinned_max_w;
                let max_scroll = (pins_content - pinned_max_w).max(0);
                pinned_scroll = pinned_scroll.clamp(0, max_scroll);

                // Pinned strip: click launches absolute paths to executables (`launch_detached`).
                if pinned_max_w > 0 && !config.pinned.apps.is_empty() {
                    let data = shm_buffer.data_mut();
                    let clip_r = layout.pinned_x + pinned_max_w;
                    for (i, app) in config.pinned.apps.iter().enumerate() {
                        let bx = layout.pinned_x + i as i32 * (layout.pin_cell + layout.pin_gap)
                            - pinned_scroll;
                        if bx + layout.pin_cell < layout.pinned_x || bx >= clip_r {
                            continue;
                        }
                        let hovered = pointer_px >= bx
                            && pointer_px < bx + layout.pin_cell
                            && pointer_py >= 2
                            && pointer_py < vis_h - 2;
                        let border = panel_rgba(config.panel.opacity, PANEL_BORDER_RGB);
                        let cell_bg = panel_rgba(
                            config.panel.opacity,
                            if hovered {
                                PANEL_ACCENT_SOFT_RGB
                            } else {
                                PANEL_SURFACE_ALT_RGB
                            },
                        );
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            bx,
                            2,
                            layout.pin_cell,
                            vis_h - 4,
                            border,
                        );
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            bx + 1,
                            3,
                            layout.pin_cell - 2,
                            vis_h - 6,
                            cell_bg,
                        );
                        let label = launcher_label(app);
                        let tx = bx
                            + (layout.pin_cell - label.len() as i32 * layout.glyph_advance) / 2;
                        draw_text(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx,
                            (vis_h - layout.glyph_h) / 2,
                            &label,
                            if hovered {
                                PANEL_TEXT_MAIN
                            } else {
                                PANEL_TEXT_MUTED
                            },
                            panel_font_scale,
                        );
                    }
                }

                let button_height = vis_h - 4;
                {
                    let data = shm_buffer.data_mut();
                    if config.workspace.enabled {
                        if workspaces.is_empty() {
                            let hint = if ipc.is_connected() { "WS?" } else { "IPC" };
                            draw_text(
                                data,
                                stride,
                                panel_width as i32,
                                ph,
                                layout.ws_start_x.max(8),
                                (vis_h - layout.glyph_h) / 2,
                                hint,
                                PANEL_TEXT_MUTED,
                                panel_font_scale,
                            );
                        } else {
                            for (i, (_id, name, focused)) in workspaces.iter().enumerate() {
                                let bx = layout.ws_start_x
                                    + (i as i32 * (layout.btn_w + 6 * panel_font_scale));
                                let by = 2;
                                let hovered = pointer_px >= bx
                                    && pointer_px < bx + layout.btn_w
                                    && pointer_py >= by
                                    && pointer_py < by + button_height;
                                let button_border =
                                    panel_rgba(config.panel.opacity, PANEL_BORDER_RGB);
                                let button_color = if *focused {
                                    PANEL_ACCENT_SOFT_RGB
                                } else if hovered {
                                    PANEL_SURFACE_ALT_RGB
                                } else {
                                    PANEL_BG_RGB
                                };
                                fill_rect(
                                    data,
                                    stride,
                                    panel_width as i32,
                                    ph,
                                    bx,
                                    by,
                                    layout.btn_w,
                                    button_height,
                                    button_border,
                                );
                                fill_rect(
                                    data,
                                    stride,
                                    panel_width as i32,
                                    ph,
                                    bx + 1,
                                    by + 1,
                                    layout.btn_w - 2,
                                    button_height - 2,
                                    panel_rgba(config.panel.opacity, button_color),
                                );
                                let text_x = bx
                                    + (layout.btn_w
                                        - name.len() as i32 * layout.glyph_advance)
                                        / 2;
                                let text_y = by + (button_height - layout.glyph_h) / 2;
                                draw_text(
                                    data,
                                    stride,
                                    panel_width as i32,
                                    ph,
                                    text_x,
                                    text_y,
                                    name,
                                    if *focused {
                                        PANEL_TEXT_MAIN
                                    } else {
                                        PANEL_TEXT_MUTED
                                    },
                                    panel_font_scale,
                                );
                            }
                        }
                    }
                }

                let stub_col = PANEL_TEXT_MUTED;
                let live_col = PANEL_ACCENT_RGB | 0xFF000000;
                let tray_cells: [(bool, &str); 3] = [
                    (config.tray.show_volume, tray_vol_lbl.as_str()),
                    (config.tray.show_updates, "U~"),
                    (config.tray.show_battery, tray_bat_lbl.as_str()),
                ];
                {
                    let data = shm_buffer.data_mut();
                    let mut tx = layout.tray_x;
                    if config.tray.show_network {
                        let cell_bg = panel_rgba(config.panel.opacity, PANEL_BG_RGB);
                        let cw = layout.tray_slot - 2;
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx,
                            2,
                            cw,
                            vis_h - 4,
                            panel_rgba(config.panel.opacity, PANEL_BORDER_RGB),
                        );
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx + 1,
                            3,
                            cw - 2,
                            vis_h - 6,
                            cell_bg,
                        );
                        draw_network_icon(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx,
                            cw,
                            vis_h,
                            tray_net_state,
                            live_col,
                            stub_col,
                            panel_font_scale,
                        );
                        tx += layout.tray_slot;
                    }
                    for (show, label) in tray_cells {
                        if !show {
                            continue;
                        }
                        let stubby = label.ends_with('~') || label == "Vm";
                        let col = if stubby { stub_col } else { live_col };
                        let cell_bg = panel_rgba(config.panel.opacity, PANEL_BG_RGB);
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx,
                            2,
                            layout.tray_slot - 2,
                            vis_h - 4,
                            panel_rgba(config.panel.opacity, PANEL_BORDER_RGB),
                        );
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx + 1,
                            3,
                            layout.tray_slot - 4,
                            vis_h - 6,
                            cell_bg,
                        );
                        draw_text(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx + 5 * panel_font_scale,
                            (vis_h - layout.glyph_h) / 2,
                            label,
                            col,
                            panel_font_scale,
                        );
                        tx += layout.tray_slot;
                    }
                }

                let clock_y = (vis_h - layout.glyph_h) / 2;
                {
                    let data = shm_buffer.data_mut();
                    let clock_w =
                        (last_clock_text.len() as i32 * layout.glyph_advance)
                            + 12 * panel_font_scale;
                    fill_rect(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        layout.clock_x - 6 * panel_font_scale,
                        2,
                        clock_w,
                        vis_h - 4,
                        panel_rgba(config.panel.opacity, PANEL_BORDER_RGB),
                    );
                    fill_rect(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        layout.clock_x - 5 * panel_font_scale,
                        3,
                        clock_w - 2,
                        vis_h - 6,
                        panel_rgba(config.panel.opacity, PANEL_SURFACE_ALT_RGB),
                    );
                    draw_text(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        layout.clock_x,
                        clock_y,
                        &last_clock_text,
                        PANEL_TEXT_MAIN,
                        panel_font_scale,
                    );
                }
            }

            surface_prepare_shm_commit(surface_id, client.socket());
            WlSurface::new(surface_id).damage_buffer(
                0,
                0,
                panel_width as i32,
                vis_h.max(1),
                client.socket(),
            );
            WlSurface::new(surface_id).commit(client.socket());
            needs_commit = false;
        }

        for event in client.poll()? {
            match event {
                Event::LayerSurfaceConfigure { serial, height, .. } => {
                    if serial != last_configure_serial {
                        last_configure_serial = serial;
                        ls.ack_configure(serial, client.socket());
                        if height > 0 {
                            layer_surface_height = height as i32;
                        }
                        needs_commit = true;
                    }
                }
                Event::LayerSurfaceClosed { .. } => return Ok(()),
                Event::PointerMotion {
                    surface_x,
                    surface_y,
                } => {
                    pointer_x = surface_x;
                    pointer_y = surface_y;
                    if config.panel.autohide
                        && layer_surface_height <= PEEK_H as i32
                        && pointer_y < (PEEK_H as f64) + 2.0
                    {
                        ls.set_size(0, panel_size_px as u32, client.socket());
                        ls.set_exclusive_zone(panel_size_px, client.socket());
                        surface_prepare_shm_commit(surface_id, client.socket());
                        WlSurface::new(surface_id).commit(client.socket());
                        layer_surface_height = buffer_draw_h;
                        autohide_collapse_at = None;
                        needs_commit = true;
                    }
                }
                Event::PointerEnter {
                    surface_x,
                    surface_y,
                } => {
                    pointer_x = surface_x;
                    pointer_y = surface_y;
                    autohide_collapse_at = None;
                    if config.panel.autohide && layer_surface_height <= PEEK_H as i32 {
                        ls.set_size(0, panel_size_px as u32, client.socket());
                        ls.set_exclusive_zone(panel_size_px, client.socket());
                        surface_prepare_shm_commit(surface_id, client.socket());
                        WlSurface::new(surface_id).commit(client.socket());
                        layer_surface_height = buffer_draw_h;
                        needs_commit = true;
                    }
                }
                Event::PointerLeave => {
                    if config.panel.autohide && layer_surface_height > PEEK_H as i32 {
                        autohide_collapse_at = Some(Instant::now() + Duration::from_millis(450));
                    }
                }
                Event::PointerAxis { axis, value } => {
                    if axis != AXIS_SCROLL_VERTICAL {
                        continue;
                    }
                    let px = pointer_x as i32;
                    let py = pointer_y as i32;
                    let vis_h = layer_surface_height.min(buffer_draw_h);
                    if vis_h <= PEEK_H as i32 {
                        continue;
                    }
                    let layout = TopBarLayout::compute(
                        panel_width as i32,
                        &last_clock_text,
                        workspaces.len(),
                        &config,
                        panel_font_scale,
                    );
                    let hit_pad = 2 * panel_font_scale;
                    if px >= layout.pinned_x
                        && px < layout.pinned_x + layout.pinned_max_w
                        && py >= hit_pad
                        && py < vis_h - hit_pad
                    {
                        pinned_scroll -= (value * 24.0) as i32;
                        needs_commit = true;
                    } else if pointer_in_tray_volume_cell(
                        px,
                        py,
                        vis_h,
                        &layout,
                        &config,
                        panel_font_scale,
                    ) {
                        // Wayland: positive axis value ≈ scroll down — invert so wheel-up increases volume.
                        let step = (-value * 3.0).round() as i32;
                        let step = step.clamp(-10, 10);
                        if step != 0 {
                            volume_nudge_by_percent(step);
                            tray_vol_lbl = tray_volume_label();
                            needs_commit = true;
                        }
                    }
                }
                Event::PointerButton { button, state } => {
                    if button == BTN_LEFT && state == 1 {
                        let px = pointer_x as i32;
                        let py = pointer_y as i32;
                        let vis_h = layer_surface_height.min(buffer_draw_h);

                        if vis_h <= PEEK_H as i32 {
                            ls.set_size(0, panel_size_px as u32, client.socket());
                            ls.set_exclusive_zone(panel_size_px, client.socket());
                            surface_prepare_shm_commit(surface_id, client.socket());
                            WlSurface::new(surface_id).commit(client.socket());
                            layer_surface_height = buffer_draw_h;
                            needs_commit = true;
                            continue;
                        }

                        let layout = TopBarLayout::compute(
                            panel_width as i32,
                            &last_clock_text,
                            workspaces.len(),
                            &config,
                            panel_font_scale,
                        );
                        let hit_pad = 2 * panel_font_scale;
                        let input_y = hit_pad;
                        let input_h = (vis_h - hit_pad * 2).max(layout.glyph_h + 2);

                        if px >= layout.input_x
                            && px < layout.input_x + layout.input_w
                            && py >= input_y
                            && py < input_y + input_h
                        {
                            if !keyboard_focused {
                                keyboard_focused = true;
                                ls.set_keyboard_interactivity(1, client.socket());
                                surface_prepare_shm_commit(surface_id, client.socket());
                                WlSurface::new(surface_id).commit(client.socket());
                            }
                            let visible_chars = ((layout.input_w - 12 * panel_font_scale)
                                / layout.glyph_advance.max(1))
                                .max(1) as usize;
                            text_input.click_at(
                                px - (layout.input_x + 6 * panel_font_scale),
                                layout.glyph_advance,
                                visible_chars,
                            );
                            needs_commit = true;
                        } else {
                            let on_vol = pointer_in_tray_volume_cell(
                                px,
                                py,
                                vis_h,
                                &layout,
                                &config,
                                panel_font_scale,
                            );
                            if on_vol {
                                volume_toggle_mute();
                                tray_vol_lbl = tray_volume_label();
                                needs_commit = true;
                            } else {
                                let pins_content = if config.pinned.apps.is_empty() {
                                    0i32
                                } else {
                                    config.pinned.apps.len() as i32
                                        * (layout.pin_cell + layout.pin_gap)
                                        - layout.pin_gap
                                };
                                let max_scroll = (pins_content - layout.pinned_max_w).max(0);
                                let scroll = pinned_scroll.clamp(0, max_scroll);

                                if layout.pinned_max_w > 0 && py >= hit_pad && py < vis_h - hit_pad {
                                    for (i, app) in config.pinned.apps.iter().enumerate() {
                                        let bx = layout.pinned_x
                                            + i as i32 * (layout.pin_cell + layout.pin_gap)
                                            - scroll;
                                        if bx + layout.pin_cell < layout.pinned_x
                                            || bx >= layout.pinned_x + layout.pinned_max_w
                                        {
                                            continue;
                                        }
                                        if px >= bx && px < bx + layout.pin_cell {
                                            launch_detached(app);
                                            break;
                                        }
                                    }
                                }

                                for (i, (id, _, _)) in workspaces.iter().enumerate() {
                                    let bx = layout.ws_start_x
                                        + i as i32 * (layout.btn_w + 6 * panel_font_scale);
                                    if px >= bx
                                        && px < bx + layout.btn_w
                                        && py >= hit_pad
                                        && py < vis_h - hit_pad
                                    {
                                        ipc.switch_workspace(*id);
                                        workspaces_raw = ipc.get_workspaces();
                                        workspaces = visible_workspaces(&workspaces_raw, &config);
                                        needs_commit = true;
                                        break;
                                    }
                                }
                            }
                            if keyboard_focused {
                                keyboard_focused = false;
                                ls.set_keyboard_interactivity(0, client.socket());
                                surface_prepare_shm_commit(surface_id, client.socket());
                                WlSurface::new(surface_id).commit(client.socket());
                                needs_commit = true;
                            }
                        }
                    }
                }
                Event::KeyboardKey { key, state, .. } => {
                    if state == 1 && keyboard_focused {
                        if key == 28 {
                            let launch_cmd = text_input.text();
                            if !launch_cmd.trim().is_empty() {
                                launch_from_input(&launch_cmd);
                            }
                            text_input.clear();
                            cursor_visible = true;
                            last_cursor_blink = Instant::now();
                            needs_commit = true;
                            continue;
                        }
                        text_input.handle_key(key);
                        cursor_visible = true;
                        last_cursor_blink = Instant::now();
                        needs_commit = true;
                    }
                }
                Event::KeyboardModifiers { mods_depressed, .. } => {
                    text_input.handle_modifiers(mods_depressed);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tray_volume_tests {
    use super::format_volume_from_amixer;
    use super::parse_amixer_master;

    #[test]
    fn parses_master_percent_and_mute() {
        let s = r"Simple mixer control 'Master',0
  Front Left: Playback 39322 [60%] [on]";
        let (muted, pct) = parse_amixer_master(s);
        assert!(!muted);
        assert_eq!(pct, Some(60));

        let m = r"  Front Left: Playback 0 [0%] [off]";
        let (muted2, _) = parse_amixer_master(m);
        assert!(muted2);
        assert_eq!(format_volume_from_amixer(m), "Vm");
    }
}
