use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use stratlayer::{
    Event, Interface,
    WlCompositor, WlDisplay, WlRegistry, WlSeat, WlShm, WlShmPool, WlSurface,
    ZwlrLayerShellV1, ZwlrLayerSurfaceV1, LAYER_TOP, ANCHOR_TOP, ANCHOR_LEFT, ANCHOR_RIGHT,
    ShmPool, ShmBuffer,
    WaylandClient,
};

mod config;
mod ipc;
mod clock;
mod textinput;

fn draw_text(buf: &mut [u8], stride: u32, panel_width: i32, panel_height: i32,
             x: i32, y: i32, text: &str, color: u32) {
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
            cursor_x += 6;
            continue;
        }

        let glyph = FONT.iter().find(|(c, _)| *c == ch);
        if let Some((_, rows)) = glyph {
            for row_idx in 0..7 {
                let row = rows[row_idx];
                for col_idx in 0..5 {
                    if (row >> (4 - col_idx)) & 1 == 1 {
                        let px = cursor_x + col_idx as i32;
                        let py = y + row_idx as i32;
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
        cursor_x += 6;
    }
}

fn fill_rect(buf: &mut [u8], stride: u32, panel_width: i32, panel_height: i32,
             x: i32, y: i32, w: i32, h: i32, color: u32) {
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
    ) -> Self {
        let input_x = 8i32;
        let input_w = 200i32;
        let pinned_left = input_x + input_w + 8;
        let clock_tw = clock_text.len() as i32 * 6;
        let tray_slot = 22i32;
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
            tray_n * tray_slot + 4
        } else {
            0i32
        };
        let clock_x = panel_width - clock_tw - 8;
        let tray_x = clock_x - 8 - tray_w;
        let pinned_x = pinned_left;
        let pinned_max_w = (tray_x - 8 - pinned_x).max(0);
        let pin_cell = 32i32;
        let pin_gap = 4i32;
        let btn_w = 40i32;
        let ws_total = ws_count as i32 * (btn_w + 4);
        let ws_start_x = (panel_width - ws_total) / 2;
        Self {
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
) -> bool {
    if py < 2 || py >= vis_h - 2 {
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
) {
    const ICON_W: i32 = 14;
    const ICON_H: i32 = 10;
    let content_h = vis_h - 4;
    let ox = cell_left + (cell_w - ICON_W) / 2;
    let oy = 2 + (content_h - ICON_H) / 2;
    match state {
        NetworkTrayState::Unknown => {
            let tx = ox + (ICON_W - 6) / 2;
            let ty = oy + (ICON_H - 7) / 2;
            draw_text(buf, stride, panel_width, panel_height, tx, ty, "?", stub);
        }
        NetworkTrayState::Connected => {
            let bar_w = 2i32;
            let gap = 2i32;
            let heights = [3i32, 5, 7, 9];
            let base = oy + ICON_H;
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
            let bar_w = 2i32;
            let gap = 2i32;
            let h = 3i32;
            let base = oy + ICON_H;
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
    let Ok(output) = Command::new("amixer").args(["-M", "get", "Master"]).output() else {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Connect to Wayland
    let mut client = WaylandClient::new()?;

    // Setup: send get_registry using allocate() so next_id advances past it
    let registry_id = client.registry().allocate();
    client.registry().set_interface(registry_id, Interface::WlRegistry);
    WlDisplay::new(1).get_registry(registry_id, client.socket());

    // Step 2: Roundtrip to collect RegistryGlobal events
    let globals = client.roundtrip()?;

    // Load configuration
    let config = config::PanelConfig::load();

    // Connect to stratvm IPC
    let mut ipc = ipc::IpcClient::connect();
    ipc.set_panel_autohide(config.panel.autohide);

    let mut compositor_name: Option<u32> = None;
    let mut shm_name: Option<u32> = None;
    let mut layer_shell_name: Option<u32> = None;
    let mut seat_name: Option<u32> = None;

    for event in &globals {
        if let Event::RegistryGlobal { name, interface, .. } = event {
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
    client.registry().set_interface(compositor_id, Interface::WlCompositor);
    WlRegistry::new(registry_id).bind(compositor_name, "wl_compositor", 4, compositor_id, client.socket());

    // Bind wl_shm
    let shm_id = client.registry().allocate();
    client.registry().set_interface(shm_id, Interface::WlShm);
    WlRegistry::new(registry_id).bind(shm_name, "wl_shm", 1, shm_id, client.socket());

    // Bind zwlr_layer_shell_v1
    let layer_shell_id = client.registry().allocate();
    // No Interface variant for ZwlrLayerShellV1 in the enum; use Unknown
    WlRegistry::new(registry_id).bind(layer_shell_name, "zwlr_layer_shell_v1", 4, layer_shell_id, client.socket());

    // Bind wl_seat
    let seat_id = client.registry().allocate();
    client.registry().set_interface(seat_id, Interface::WlSeat);
    let seat_name = seat_name.ok_or("wl_seat not found")?;
    WlRegistry::new(registry_id).bind(seat_name, "wl_seat", 7, seat_id, client.socket());

    // Get pointer from seat
    let pointer_id = client.registry().allocate();
    client.registry().set_interface(pointer_id, Interface::WlPointer);
    WlSeat::new(seat_id).get_pointer(pointer_id, client.socket());

    // Get keyboard from seat
    let keyboard_id = client.registry().allocate();
    client.registry().set_interface(keyboard_id, Interface::WlKeyboard);
    WlSeat::new(seat_id).get_keyboard(keyboard_id, client.socket());

    // Step 3: Create wl_surface
    let surface_id = client.registry().allocate();
    client.registry().set_interface(surface_id, Interface::WlSurface);
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
    ls.set_size(0, config.panel.size, client.socket());
    ls.set_anchor(ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT, client.socket());
    ls.set_exclusive_zone(config.panel.size as i32, client.socket());
    ls.set_keyboard_interactivity(1, client.socket());
    WlSurface::new(surface_id).commit(client.socket());

    // Step 6: Wait for LayerSurfaceConfigure
    let (confirmed_width, confirmed_height);
    'configure: loop {
        for event in client.poll()? {
            if let Event::LayerSurfaceConfigure { serial, width, height, .. } = event {
                ls.ack_configure(serial, client.socket());
                confirmed_width = width;
                confirmed_height = height;
                break 'configure;
            }
        }
    }

    // Step 7: Allocate SHM buffer (full design height; compositor clips when autohide uses PEEK_H)
    let panel_width = if confirmed_width == 0 { 1920 } else { confirmed_width };
    let buffer_draw_h = config.panel.size as i32;
    let stride = panel_width * 4;
    let size = (stride * buffer_draw_h as u32) as usize;

    let pool = ShmPool::create(size)?;
    let shm_fd = pool.fd();

    // Fill with configurable-opacity panel background (ARGB8888)
    let mut shm_buffer = ShmBuffer::new(pool, 0, panel_width, buffer_draw_h as u32, stride);
    {
        let data = shm_buffer.data_mut();
        let color = ((config.panel.opacity * 255.0) as u32) << 24 | 0x2B2B2B;
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
    client.registry().set_interface(pool_id, Interface::WlShmPool);
    WlShm::new(shm_id).create_pool(pool_id, shm_fd, size as i32, client.socket());

    // Create wl_buffer from pool
    let buffer_id = client.registry().allocate();
    client.registry().set_interface(buffer_id, Interface::WlBuffer);
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
    WlSurface::new(surface_id).attach(buffer_id, 0, 0, client.socket());
    WlSurface::new(surface_id).damage(0, 0, panel_width as i32, buffer_draw_h, client.socket());
    WlSurface::new(surface_id).commit(client.socket());

    let mut layer_surface_height = if confirmed_height == 0 {
        buffer_draw_h
    } else {
        confirmed_height as i32
    };

    if config.panel.autohide {
        ls.set_size(0, PEEK_H, client.socket());
        ls.set_exclusive_zone(0, client.socket());
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
                let color = ((config.panel.opacity * 255.0) as u32) << 24 | 0x2B2B2B;
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
                let accent = ((config.panel.opacity * 255.0) as u32) << 24 | 0x5B9BD5;
                fill_rect(data, stride, panel_width as i32, ph, 0, 0, panel_width as i32, vis_h, accent);
            } else {
                let layout = TopBarLayout::compute(
                    panel_width as i32,
                    &last_clock_text,
                    workspaces.len(),
                    &config,
                );
                let input_y = 2i32;
                let input_h = vis_h - 4;

                {
                    let data = shm_buffer.data_mut();
                    let input_bg = ((config.panel.opacity * 255.0) as u32) << 24 | 0x1B1B1B;
                    fill_rect(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        layout.input_x,
                        input_y,
                        layout.input_w,
                        input_h,
                        input_bg,
                    );

                    let display_text = text_input.display_text();
                    let text_y = input_y + (input_h - 7) / 2;
                    draw_text(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        layout.input_x + 4,
                        text_y,
                        &display_text,
                        0xFFFFFFFF,
                    );

                    if keyboard_focused && cursor_visible {
                        let cursor_x = layout.input_x + 4 + text_input.cursor_pixel_offset();
                        fill_rect(data, stride, panel_width as i32, ph, cursor_x, text_y, 1, 9, 0xFFFFFFFF);
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
                        let bx = layout.pinned_x
                            + i as i32 * (layout.pin_cell + layout.pin_gap)
                            - pinned_scroll;
                        if bx + layout.pin_cell < layout.pinned_x || bx >= clip_r {
                            continue;
                        }
                        let cell_bg = ((config.panel.opacity * 255.0) as u32) << 24 | 0x252525;
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            bx,
                            2,
                            layout.pin_cell,
                            vis_h - 4,
                            cell_bg,
                        );
                        let label = launcher_label(app);
                        let tx = bx + (layout.pin_cell - label.len() as i32 * 6) / 2;
                        draw_text(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx,
                            (vis_h - 7) / 2,
                            &label,
                            0xFFE0E0E0,
                        );
                    }
                }

                let button_height = vis_h - 4;
                {
                    let data = shm_buffer.data_mut();
                    if config.workspace.enabled {
                        if workspaces.is_empty() {
                            let hint = if ipc.is_connected() {
                                "WS?"
                            } else {
                                "IPC"
                            };
                            draw_text(
                                data,
                                stride,
                                panel_width as i32,
                                ph,
                                layout.ws_start_x.max(8),
                                (vis_h - 7) / 2,
                                hint,
                                0xFF666666,
                            );
                        } else {
                            for (i, (_id, name, focused)) in workspaces.iter().enumerate() {
                            let bx =
                                layout.ws_start_x + (i as i32 * (layout.btn_w + 4));
                            let by = 2;
                            let button_color = if *focused {
                                ((config.panel.opacity * 255.0) as u32) << 24 | 0x3B3B3B
                            } else {
                                ((config.panel.opacity * 255.0) as u32) << 24 | 0x1B1B1B
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
                                button_color,
                            );
                            let text_x = bx + (layout.btn_w - name.len() as i32 * 6) / 2;
                            let text_y = by + (button_height - 7) / 2;
                            draw_text(
                                data,
                                stride,
                                panel_width as i32,
                                ph,
                                text_x,
                                text_y,
                                name,
                                0xFFFFFFFF,
                            );
                        }
                        }
                    }
                }

                let stub_col = 0xFF666666u32;
                let live_col = 0xFF9BB9B9u32;
                let tray_cells: [(bool, &str); 3] = [
                    (config.tray.show_volume, tray_vol_lbl.as_str()),
                    (config.tray.show_updates, "U~"),
                    (config.tray.show_battery, tray_bat_lbl.as_str()),
                ];
                {
                    let data = shm_buffer.data_mut();
                    let mut tx = layout.tray_x;
                    if config.tray.show_network {
                        let cell_bg = ((config.panel.opacity * 255.0) as u32) << 24 | 0x151515;
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
                        );
                        tx += layout.tray_slot;
                    }
                    for (show, label) in tray_cells {
                        if !show {
                            continue;
                        }
                        let stubby = label.ends_with('~') || label == "Vm";
                        let col = if stubby { stub_col } else { live_col };
                        let cell_bg = ((config.panel.opacity * 255.0) as u32) << 24 | 0x151515;
                        fill_rect(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx,
                            2,
                            layout.tray_slot - 2,
                            vis_h - 4,
                            cell_bg,
                        );
                        draw_text(
                            data,
                            stride,
                            panel_width as i32,
                            ph,
                            tx + 4,
                            (vis_h - 7) / 2,
                            label,
                            col,
                        );
                        tx += layout.tray_slot;
                    }
                }

                let clock_y = (vis_h - 7) / 2;
                {
                    let data = shm_buffer.data_mut();
                    draw_text(
                        data,
                        stride,
                        panel_width as i32,
                        ph,
                        layout.clock_x,
                        clock_y,
                        &last_clock_text,
                        0xFFFFFFFF,
                    );
                }
            }

            WlSurface::new(surface_id).damage(0, 0, panel_width as i32, vis_h.max(1), client.socket());
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
                Event::PointerMotion { surface_x, surface_y } => {
                    pointer_x = surface_x;
                    pointer_y = surface_y;
                    if config.panel.autohide
                        && layer_surface_height <= PEEK_H as i32
                        && pointer_y < (PEEK_H as f64) + 2.0
                    {
                        ls.set_size(0, config.panel.size, client.socket());
                        ls.set_exclusive_zone(config.panel.size as i32, client.socket());
                        WlSurface::new(surface_id).commit(client.socket());
                        layer_surface_height = buffer_draw_h;
                        autohide_collapse_at = None;
                        needs_commit = true;
                    }
                }
                Event::PointerEnter { surface_x, surface_y } => {
                    pointer_x = surface_x;
                    pointer_y = surface_y;
                    autohide_collapse_at = None;
                    if config.panel.autohide && layer_surface_height <= PEEK_H as i32 {
                        ls.set_size(0, config.panel.size, client.socket());
                        ls.set_exclusive_zone(config.panel.size as i32, client.socket());
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
                    );
                    if px >= layout.pinned_x
                        && px < layout.pinned_x + layout.pinned_max_w
                        && py >= 2
                        && py < vis_h - 2
                    {
                        pinned_scroll -= (value * 24.0) as i32;
                        needs_commit = true;
                    } else if pointer_in_tray_volume_cell(px, py, vis_h, &layout, &config) {
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
                            ls.set_size(0, config.panel.size, client.socket());
                            ls.set_exclusive_zone(config.panel.size as i32, client.socket());
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
                        );

                        if px >= layout.input_x
                            && px < layout.input_x + layout.input_w
                            && py >= 2
                            && py < vis_h - 2
                        {
                            if !keyboard_focused {
                                keyboard_focused = true;
                                ls.set_keyboard_interactivity(1, client.socket());
                                WlSurface::new(surface_id).commit(client.socket());
                            }
                            text_input.click_at(px - (layout.input_x + 4));
                        } else {
                            let on_vol = pointer_in_tray_volume_cell(px, py, vis_h, &layout, &config);
                            if on_vol {
                                volume_toggle_mute();
                                tray_vol_lbl = tray_volume_label();
                                needs_commit = true;
                            } else {
                                let pins_content = if config.pinned.apps.is_empty() {
                                    0i32
                                } else {
                                    config.pinned.apps.len() as i32 * (layout.pin_cell + layout.pin_gap)
                                        - layout.pin_gap
                                };
                                let max_scroll = (pins_content - layout.pinned_max_w).max(0);
                                let scroll = pinned_scroll.clamp(0, max_scroll);

                                if layout.pinned_max_w > 0 && py >= 2 && py < vis_h - 2 {
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
                                    let bx = layout.ws_start_x + i as i32 * (layout.btn_w + 4);
                                    if px >= bx
                                        && px < bx + layout.btn_w
                                        && py >= 2
                                        && py < vis_h - 2
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
                                WlSurface::new(surface_id).commit(client.socket());
                            }
                        }
                    }
                }
                Event::KeyboardKey { key, state, .. } => {
                    if state == 1 && keyboard_focused {
                        text_input.handle_key(key);
                        cursor_visible = true;
                        last_cursor_blink = Instant::now();
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
