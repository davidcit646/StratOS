//! StratOS Settings — search-first Wayland client (`xdg-shell`) for `/config/strat/settings.toml`.
// See `docs/agent/stratsettings.md` and Phase 26 checklist.

mod font;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use stratlayer::{
    Event, Interface, WaylandClient, WlCompositor, WlDisplay, WlRegistry, WlSeat, WlShm, WlShmPool,
    WlSurface, XdgSurface, XdgToplevel, XdgWmBase, ShmBuffer, ShmPool,
};
use stratsettings::{StratSettings, CONFIG_DIR};

use font::{draw_text, fill_rect};

const MOD_CTRL: u32 = 1 << 2;

const BG: u32 = 0xFF2B2B2B;
const FG: u32 = 0xFFF0F0F0;
const ACCENT: u32 = 0xFF5B9BD5;
const HI: u32 = 0xFF3D5270;
const BTN_LEFT: u32 = 0x110;

const ROW_H: i32 = 18;
const SEARCH_Y: i32 = 52;
const LIST_Y: i32 = 84;
const FOOTER_H: i32 = 22;

/// Every user-visible row: label, search keywords, value line, apply mutation.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowId {
    PanelAutohide,
    PanelSize,
    PanelOpacity,
    PanelBlur,
    ClockFormat,
    ClockShowDate,
    TrayNet,
    TrayVol,
    TrayUpd,
    TrayBat,
    TermStatusBar,
    TermTitleBar,
    ChromeTitleH,
    ChromeBorderPad,
    ChromeDecoDefault,
    SpotHeadlessEnabled,
    SpotHeadlessBoot,
    SpotRescanSecs,
    SpotHeadlessBatch,
    SpotUiEnabled,
    SpotUiTickMs,
    SpotHotkeyDisplay,
}

struct RowDef {
    id: RowId,
    label: &'static str,
    keywords: &'static str,
}

const ROWS: &[RowDef] = &[
    RowDef {
        id: RowId::PanelAutohide,
        label: "Panel auto-hide",
        keywords: "panel autohide hide dock bar",
    },
    RowDef {
        id: RowId::PanelSize,
        label: "Panel height (px)",
        keywords: "panel size height pixels",
    },
    RowDef {
        id: RowId::PanelOpacity,
        label: "Panel opacity",
        keywords: "panel opacity transparent",
    },
    RowDef {
        id: RowId::PanelBlur,
        label: "Panel blur (reserved)",
        keywords: "panel blur",
    },
    RowDef {
        id: RowId::ClockFormat,
        label: "Clock format",
        keywords: "clock time 12 24 hour",
    },
    RowDef {
        id: RowId::ClockShowDate,
        label: "Clock show date",
        keywords: "clock date calendar",
    },
    RowDef {
        id: RowId::TrayNet,
        label: "Tray: network",
        keywords: "tray network wifi ethernet",
    },
    RowDef {
        id: RowId::TrayVol,
        label: "Tray: volume",
        keywords: "tray volume audio sound mixer",
    },
    RowDef {
        id: RowId::TrayUpd,
        label: "Tray: updates",
        keywords: "tray updates software",
    },
    RowDef {
        id: RowId::TrayBat,
        label: "Tray: battery",
        keywords: "tray battery power",
    },
    RowDef {
        id: RowId::TermStatusBar,
        label: "Stratterm: explorer status bar",
        keywords: "stratterm terminal explorer status",
    },
    RowDef {
        id: RowId::TermTitleBar,
        label: "Stratterm: title bar split",
        keywords: "stratterm title bar chrome files",
    },
    RowDef {
        id: RowId::ChromeTitleH,
        label: "Titlebar height (px)",
        keywords: "window decoration chrome height titlebar",
    },
    RowDef {
        id: RowId::ChromeBorderPad,
        label: "Window border padding (px)",
        keywords: "border padding frame gap chrome",
    },
    RowDef {
        id: RowId::ChromeDecoDefault,
        label: "Decorations on by default",
        keywords: "window decorations border",
    },
    RowDef {
        id: RowId::SpotHeadlessEnabled,
        label: "Spotlite: indexer enabled",
        keywords: "spotlite indexer sqlite path-index scan",
    },
    RowDef {
        id: RowId::SpotHeadlessBoot,
        label: "Spotlite: indexer at boot",
        keywords: "spotlite daemon boot stratterm-indexer",
    },
    RowDef {
        id: RowId::SpotRescanSecs,
        label: "Spotlite: rescan interval (sec)",
        keywords: "spotlite rescan interval seconds",
    },
    RowDef {
        id: RowId::SpotHeadlessBatch,
        label: "Spotlite: indexer batch size",
        keywords: "spotlite batch index chunk",
    },
    RowDef {
        id: RowId::SpotUiEnabled,
        label: "Spotlite: UI hints enabled",
        keywords: "spotlite ui stratterm overlay idle",
    },
    RowDef {
        id: RowId::SpotUiTickMs,
        label: "Spotlite: UI tick (ms)",
        keywords: "spotlite tick timer ms",
    },
    RowDef {
        id: RowId::SpotHotkeyDisplay,
        label: "Spotlite: compositor hotkey (read-only)",
        keywords: "spotlite keyboard super period stratvm keybind",
    },
];

fn row_value(s: &StratSettings, id: RowId) -> String {
    match id {
        RowId::PanelAutohide => format!("{}", s.panel.autohide),
        RowId::PanelSize => format!("{}", s.panel.size),
        RowId::PanelOpacity => format!("{:.2}", s.panel.opacity),
        RowId::PanelBlur => format!("{}", s.panel.blur),
        RowId::ClockFormat => s.panel.clock.format.clone(),
        RowId::ClockShowDate => format!("{}", s.panel.clock.show_date),
        RowId::TrayNet => format!("{}", s.panel.tray.show_network),
        RowId::TrayVol => format!("{}", s.panel.tray.show_volume),
        RowId::TrayUpd => format!("{}", s.panel.tray.show_updates),
        RowId::TrayBat => format!("{}", s.panel.tray.show_battery),
        RowId::TermStatusBar => format!("{}", s.stratterm.file_explorer.status_bar_enabled),
        RowId::TermTitleBar => format!("{}", s.stratterm.file_explorer.client_title_bar_enabled),
        RowId::ChromeTitleH => format!("{}", s.chrome.decoration_titlebar_height),
        RowId::ChromeBorderPad => format!("{}", s.chrome.border_pad),
        RowId::ChromeDecoDefault => format!("{}", s.chrome.decorations_enabled_default),
        RowId::SpotHeadlessEnabled => format!("{}", s.spotlite.headless.enabled),
        RowId::SpotHeadlessBoot => format!("{}", s.spotlite.headless.boot_start),
        RowId::SpotRescanSecs => format!("{}", s.spotlite.headless.rescan_secs),
        RowId::SpotHeadlessBatch => format!("{}", s.spotlite.headless.batch_limit),
        RowId::SpotUiEnabled => format!("{}", s.spotlite.ui.enabled),
        RowId::SpotUiTickMs => format!("{}", s.spotlite.ui.tick_ms),
        RowId::SpotHotkeyDisplay => s.keyboard.spotlite.clone(),
    }
}

fn row_toggle(s: &mut StratSettings, id: RowId) {
    match id {
        RowId::PanelAutohide => s.panel.autohide = !s.panel.autohide,
        RowId::PanelBlur => s.panel.blur = !s.panel.blur,
        RowId::ClockShowDate => s.panel.clock.show_date = !s.panel.clock.show_date,
        RowId::TrayNet => s.panel.tray.show_network = !s.panel.tray.show_network,
        RowId::TrayVol => s.panel.tray.show_volume = !s.panel.tray.show_volume,
        RowId::TrayUpd => s.panel.tray.show_updates = !s.panel.tray.show_updates,
        RowId::TrayBat => s.panel.tray.show_battery = !s.panel.tray.show_battery,
        RowId::TermStatusBar => {
            s.stratterm.file_explorer.status_bar_enabled = !s.stratterm.file_explorer.status_bar_enabled;
        }
        RowId::TermTitleBar => {
            s.stratterm.file_explorer.client_title_bar_enabled =
                !s.stratterm.file_explorer.client_title_bar_enabled;
        }
        RowId::ChromeDecoDefault => {
            s.chrome.decorations_enabled_default = !s.chrome.decorations_enabled_default;
        }
        RowId::SpotHeadlessEnabled => {
            s.spotlite.headless.enabled = !s.spotlite.headless.enabled;
        }
        RowId::SpotHeadlessBoot => {
            s.spotlite.headless.boot_start = !s.spotlite.headless.boot_start;
        }
        RowId::SpotUiEnabled => {
            s.spotlite.ui.enabled = !s.spotlite.ui.enabled;
        }
        RowId::SpotHotkeyDisplay => {}
        RowId::ClockFormat => {
            s.panel.clock.format = if s.panel.clock.format == "24hr" {
                "12hr".into()
            } else {
                "24hr".into()
            };
        }
        RowId::PanelSize
        | RowId::PanelOpacity
        | RowId::ChromeTitleH
        | RowId::ChromeBorderPad
        | RowId::SpotRescanSecs
        | RowId::SpotHeadlessBatch
        | RowId::SpotUiTickMs => {}
    }
}

fn row_adjust(s: &mut StratSettings, id: RowId, delta: i32) {
    match id {
        RowId::PanelSize => {
            let v = s.panel.size as i32 + delta;
            s.panel.size = (v.clamp(20, 64) / 2 * 2) as u32;
        }
        RowId::PanelOpacity => {
            let mut o = s.panel.opacity + (delta as f64) * 0.05;
            if o < 0.3 {
                o = 0.3;
            }
            if o > 1.0 {
                o = 1.0;
            }
            s.panel.opacity = (o * 100.0).round() / 100.0;
        }
        RowId::ChromeTitleH => {
            let v = s.chrome.decoration_titlebar_height as i32 + delta;
            s.chrome.decoration_titlebar_height = v.clamp(12, 64) as u32;
        }
        RowId::ChromeBorderPad => {
            let v = s.chrome.border_pad as i32 + delta;
            s.chrome.border_pad = v.clamp(0, 12) as u32;
        }
        RowId::SpotRescanSecs => {
            let step = 10i64 * delta as i64;
            let v = s.spotlite.headless.rescan_secs as i64 + step;
            s.spotlite.headless.rescan_secs = v.clamp(30, 86_400) as u64;
        }
        RowId::SpotHeadlessBatch => {
            let v = s.spotlite.headless.batch_limit as i32 + delta * 4;
            s.spotlite.headless.batch_limit = v.clamp(16, 512) as usize;
        }
        RowId::SpotUiTickMs => {
            let v = s.spotlite.ui.tick_ms as i32 + delta * 50;
            s.spotlite.ui.tick_ms = v.clamp(200, 10_000) as u64;
        }
        _ => {}
    }
}

fn filter_rows(query: &str) -> Vec<usize> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return (0..ROWS.len()).collect();
    }
    ROWS
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.label.to_ascii_lowercase().contains(&q)
                || r.keywords.to_ascii_lowercase().contains(&q)
        })
        .map(|(i, _)| i)
        .collect()
}

struct TextInput {
    buf: Vec<char>,
    cursor: usize,
}

impl TextInput {
    fn new() -> Self {
        TextInput {
            buf: Vec::new(),
            cursor: 0,
        }
    }

    fn text(&self) -> String {
        self.buf.iter().collect()
    }

    fn handle_key(&mut self, key: u32) {
        match key {
            14 => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.buf.remove(self.cursor);
                }
            }
            105 => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            106 => {
                if self.cursor < self.buf.len() {
                    self.cursor += 1;
                }
            }
            1 => {
                self.buf.clear();
                self.cursor = 0;
            }
            _ => {
                if let Some(c) = map_printable(key, false, false) {
                    if self.buf.len() < 120 {
                        self.buf.insert(self.cursor, c);
                        self.cursor += 1;
                    }
                }
            }
        }
    }
}

fn map_printable(key: u32, shift: bool, _caps: bool) -> Option<char> {
    let upper = shift;
    match key {
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
        30 => Some(if upper { 'A' } else { 'a' }),
        31 => Some(if upper { 'S' } else { 's' }),
        32 => Some(if upper { 'D' } else { 'd' }),
        33 => Some(if upper { 'F' } else { 'f' }),
        34 => Some(if upper { 'G' } else { 'g' }),
        35 => Some(if upper { 'H' } else { 'h' }),
        36 => Some(if upper { 'J' } else { 'j' }),
        37 => Some(if upper { 'K' } else { 'k' }),
        38 => Some(if upper { 'L' } else { 'l' }),
        44 => Some(if upper { 'Z' } else { 'z' }),
        45 => Some(if upper { 'X' } else { 'x' }),
        46 => Some(if upper { 'C' } else { 'c' }),
        47 => Some(if upper { 'V' } else { 'v' }),
        48 => Some(if upper { 'B' } else { 'b' }),
        49 => Some(if upper { 'N' } else { 'n' }),
        50 => Some(if upper { 'M' } else { 'm' }),
        2..=11 => {
            let ch = match key {
                11 => '0',
                k => char::from(b'1' + (k - 2) as u8),
            };
            Some(if upper {
                match key {
                    11 => ')' as char,
                    2 => '!' as char,
                    3 => '@' as char,
                    4 => '#' as char,
                    5 => '$' as char,
                    6 => '%' as char,
                    7 => '^' as char,
                    8 => '&' as char,
                    9 => '*' as char,
                    10 => '(' as char,
                    _ => ch,
                }
            } else {
                ch
            })
        }
        12 => Some(if upper { '_' } else { '-' }),
        13 => Some(if upper { '+' } else { '=' }),
        57 => Some(' '),
        _ => None,
    }
}

fn stratvm_autohide(on: bool) {
    let Ok(mut s) = UnixStream::connect("/run/stratvm.sock") else {
        return;
    };
    let cmd = format!("set panel autohide {}\n", on);
    let _ = s.write_all(cmd.as_bytes());
}

fn stratvm_reload_keybinds() {
    let Ok(mut s) = UnixStream::connect("/run/stratvm.sock") else {
        return;
    };
    let _ = s.write_all(b"reload_keybinds\n");
}

fn stratvm_ping_ok() -> bool {
    let Ok(mut s) = UnixStream::connect("/run/stratvm.sock") else {
        return false;
    };
    let _ = s.write_all(b"ping\n");
    let mut r = String::new();
    let mut br = BufReader::new(s);
    br.read_line(&mut r).ok();
    r.trim() == "OK pong"
}

fn save_settings(
    settings: &StratSettings,
    config_root: &Path,
    footer: &mut String,
) -> Result<(), String> {
    std::fs::create_dir_all(config_root).map_err(|e| e.to_string())?;
    settings.save_to(config_root)?;
    stratvm_autohide(settings.panel.autohide);
    stratvm_reload_keybinds();
    *footer = if stratvm_ping_ok() {
        "Saved. panel + compositor notified.".into()
    } else {
        "Saved to disk (stratvm socket missing?).".into()
    };
    Ok(())
}

fn draw_frame(
    buf: &mut [u8],
    stride: u32,
    w: i32,
    h: i32,
    settings: &StratSettings,
    search: &TextInput,
    focus_search: bool,
    filtered: &[usize],
    sel: usize,
    scroll: usize,
    footer: &str,
) {
    fill_rect(buf, stride, w, h, 0, 0, w, h, BG);
    draw_text(
        buf, stride, w, h, 8, 8,
        "StratOS Settings — type to filter, Tab switch, Enter toggle",
        FG,
    );
    let hint = if focus_search {
        "Search [active]"
    } else {
        "List [active]"
    };
    draw_text(buf, stride, w, h, 8, 28, hint, ACCENT);

    let q = search.text();
    let line = format!("> {}", q);
    draw_text(buf, stride, w, h, 8, SEARCH_Y, &line, FG);

    let max_rows = ((h - LIST_Y - FOOTER_H) / ROW_H).max(1) as usize;
    let start = scroll.min(filtered.len().saturating_sub(1));
    let end = (start + max_rows).min(filtered.len());

    for (vis, &idx) in filtered[start..end].iter().enumerate() {
        let r = &ROWS[idx];
        let y = LIST_Y + vis as i32 * ROW_H;
        let row_idx = start + vis;
        let hl = !focus_search && row_idx == sel;
        if hl {
            fill_rect(buf, stride, w, h, 4, y - 2, w - 8, ROW_H, HI);
        }
        let val = row_value(settings, r.id);
        let mut left = String::from(r.label);
        left.push_str("  ");
        draw_text(buf, stride, w, h, 12, y, &left, FG);
        let vx = w - 12 - (val.len() as i32 * 6).min(w / 2);
        draw_text(buf, stride, w, h, vx.max(200), y, &val, if hl { FG } else { ACCENT });
    }

    let foot = if footer.len() > 100 {
        &footer[..100]
    } else {
        footer
    };
    draw_text(
        buf,
        stride,
        w,
        h,
        8,
        h - FOOTER_H / 2 - 4,
        foot,
        0xFFAAAAAA,
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_root = std::env::var("STRAT_CONFIG_ROOT")
        .map(|s| Path::new(&s).to_path_buf())
        .unwrap_or_else(|_| Path::new(CONFIG_DIR).to_path_buf());

    let mut settings = StratSettings::load_from(&config_root).unwrap_or_else(|_| StratSettings::default());

    let mut client = WaylandClient::new()?;
    let registry_id = client.registry().allocate();
    client.registry().set_interface(registry_id, Interface::WlRegistry);
    WlDisplay::new(1).get_registry(registry_id, client.socket());
    let globals = client.roundtrip()?;

    let mut compositor_n: Option<u32> = None;
    let mut shm_n: Option<u32> = None;
    let mut xdg_n: Option<u32> = None;
    let mut seat_n: Option<u32> = None;

    for event in &globals {
        if let Event::RegistryGlobal { name, interface, .. } = event {
            match interface.as_str() {
                "wl_compositor" => compositor_n = Some(*name),
                "wl_shm" => shm_n = Some(*name),
                "xdg_wm_base" => xdg_n = Some(*name),
                "wl_seat" => seat_n = Some(*name),
                _ => {}
            }
        }
    }

    let compositor_n = compositor_n.ok_or("wl_compositor missing")?;
    let shm_n = shm_n.ok_or("wl_shm missing")?;
    let xdg_n = xdg_n.ok_or("xdg_wm_base missing")?;

    let compositor_id = client.registry().allocate();
    client.registry().set_interface(compositor_id, Interface::WlCompositor);
    WlRegistry::new(registry_id).bind(compositor_n, "wl_compositor", 4, compositor_id, client.socket());

    let shm_id = client.registry().allocate();
    client.registry().set_interface(shm_id, Interface::WlShm);
    WlRegistry::new(registry_id).bind(shm_n, "wl_shm", 1, shm_id, client.socket());

    let xdg_wm_base_id = client.registry().allocate();
    client.registry().set_interface(xdg_wm_base_id, Interface::XdgWmBase);
    WlRegistry::new(registry_id).bind(xdg_n, "xdg_wm_base", 1, xdg_wm_base_id, client.socket());

    let seat_id = client.registry().allocate();
    client.registry().set_interface(seat_id, Interface::WlSeat);
    let seat_n = seat_n.ok_or("wl_seat missing")?;
    WlRegistry::new(registry_id).bind(seat_n, "wl_seat", 7, seat_id, client.socket());

    let pointer_id = client.registry().allocate();
    client.registry().set_interface(pointer_id, Interface::WlPointer);
    WlSeat::new(seat_id).get_pointer(pointer_id, client.socket());

    let keyboard_id = client.registry().allocate();
    client.registry().set_interface(keyboard_id, Interface::WlKeyboard);
    WlSeat::new(seat_id).get_keyboard(keyboard_id, client.socket());

    let surface_id = client.registry().allocate();
    client.registry().set_interface(surface_id, Interface::WlSurface);
    WlCompositor::new(compositor_id).create_surface(surface_id, client.socket());

    let xdg_surface_id = client.registry().allocate();
    client.registry().set_interface(xdg_surface_id, Interface::XdgSurface);
    XdgWmBase::new(xdg_wm_base_id).get_xdg_surface(xdg_surface_id, surface_id, client.socket());

    let toplevel_id = client.registry().allocate();
    client.registry().set_interface(toplevel_id, Interface::XdgToplevel);
    XdgSurface::new(xdg_surface_id).get_toplevel(toplevel_id, client.socket());
    let toplevel = XdgToplevel::new(toplevel_id);
    toplevel.set_title("StratOS Settings", client.socket());
    toplevel.set_app_id("org.stratos.settings", client.socket());

    WlSurface::new(surface_id).commit(client.socket());

    let win_w: i32 = 720;
    let win_h: i32 = 520;

    let xdg_serial = 'cfg: loop {
        for e in client.poll()? {
            if let Event::XdgSurfaceConfigure { serial, .. } = e {
                break 'cfg serial;
            }
            if let Event::XdgPing { serial } = e {
                XdgWmBase::new(xdg_wm_base_id).pong(serial, client.socket());
            }
        }
    };
    XdgSurface::new(xdg_surface_id).ack_configure(xdg_serial, client.socket());

    let stride = win_w * 4;
    let bufsize = (stride * win_h) as usize;
    let pool = ShmPool::create(bufsize)?;
    let pool_id = client.registry().allocate();
    client.registry().set_interface(pool_id, Interface::WlShmPool);
    WlShm::new(shm_id).create_pool(pool_id, pool.fd(), bufsize as i32, client.socket());

    let buffer_id = client.registry().allocate();
    client.registry().set_interface(buffer_id, Interface::WlBuffer);
    WlShmPool::new(pool_id).create_buffer(
        buffer_id,
        0,
        win_w,
        win_h,
        stride,
        0,
        client.socket(),
    );

    let mut shm_buffer = ShmBuffer::new(pool, 0, win_w as u32, win_h as u32, stride as u32);

    let mut search = TextInput::new();
    let mut focus_search = true;
    let mut sel: usize = 0;
    let mut scroll: usize = 0;
    let mut filtered = filter_rows("");
    let mut footer = String::from("Ctrl+S save  |  Tab search/list  |  +/- adjust numbers");
    let mut mods = 0u32;
    let mut needs = true;
    loop {
        if needs {
            let data = shm_buffer.data_mut();
            draw_frame(
                data,
                stride as u32,
                win_w,
                win_h,
                &settings,
                &search,
                focus_search,
                &filtered,
                sel,
                scroll,
                &footer,
            );
            WlSurface::new(surface_id).attach(buffer_id, 0, 0, client.socket());
            WlSurface::new(surface_id)
                .damage(0, 0, win_w, win_h, client.socket());
            WlSurface::new(surface_id).commit(client.socket());
            needs = false;
        }

        let events = client.poll()?;
        for event in events {
            match event {
                Event::XdgPing { serial } => {
                    XdgWmBase::new(xdg_wm_base_id).pong(serial, client.socket());
                }
                Event::XdgSurfaceConfigure { serial, .. } => {
                    XdgSurface::new(xdg_surface_id).ack_configure(serial, client.socket());
                    needs = true;
                }
                Event::XdgToplevelConfigure { .. } => {}
                Event::XdgToplevelClose => std::process::exit(0),
                Event::KeyboardModifiers { mods_depressed, .. } => {
                    mods = mods_depressed;
                }
                Event::KeyboardKey { key, state, .. } => {
                    if state != 1 {
                        continue;
                    }
                    let ctrl = (mods & MOD_CTRL) != 0;
                    if ctrl && key == 31 {
                        // Ctrl+S (KEY_S); KEY_C is 46 — do not treat Ctrl+C as save
                        if save_settings(&settings, &config_root, &mut footer).is_err() {
                            footer = "Save failed (permissions?)".into();
                        }
                        needs = true;
                        continue;
                    }
                    if key == 15 {
                        // Tab
                        focus_search = !focus_search;
                        filtered = filter_rows(&search.text());
                        sel = sel.min(filtered.len().saturating_sub(1));
                        needs = true;
                        continue;
                    }
                    if key == 103 && !focus_search {
                        if sel > 0 {
                            sel -= 1;
                            if sel < scroll {
                                scroll = sel;
                            }
                        }
                        needs = true;
                        continue;
                    }
                    if key == 108 && !focus_search {
                        if sel + 1 < filtered.len() {
                            sel += 1;
                            let max_rows = ((win_h - LIST_Y - FOOTER_H) / ROW_H).max(1) as usize;
                            if sel >= scroll + max_rows {
                                scroll = sel.saturating_sub(max_rows - 1);
                            }
                        }
                        needs = true;
                        continue;
                    }
                    if focus_search {
                        search.handle_key(key);
                        filtered = filter_rows(&search.text());
                        sel = sel.min(filtered.len().saturating_sub(1));
                        needs = true;
                        continue;
                    }
                    // List
                    if key == 28 || key == 57 {
                        if let Some(&fidx) = filtered.get(sel) {
                            let id = ROWS[fidx].id;
                            row_toggle(&mut settings, id);
                            needs = true;
                        }
                        continue;
                    }
                    if key == 12 || key == 13 {
                        let d = if key == 13 { 1 } else { -1 };
                        if let Some(&fidx) = filtered.get(sel) {
                            row_adjust(&mut settings, ROWS[fidx].id, d);
                            needs = true;
                        }
                    }
                }
                Event::PointerButton { button, state, .. } => {
                    if button == BTN_LEFT && state == 1 {
                        // Pixel coords not tracked in this minimal port
                        needs = true;
                    }
                }
                _ => {}
            }
        }
    }
}
