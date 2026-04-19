//! StratOS Spotlite — fullscreen layer overlay: search `path-index.db`, launch apps / `xdg-open`.

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rusqlite::{params, Connection};
use stratlayer::{
    Event, Interface, ShmBuffer, ShmPool, WaylandClient, WlCompositor, WlDisplay, WlRegistry, WlSeat,
    WlShm, WlShmPool, WlSurface, ZwlrLayerShellV1, ZwlrLayerSurfaceV1, LAYER_OVERLAY, ANCHOR_BOTTOM,
    ANCHOR_LEFT, ANCHOR_RIGHT, ANCHOR_TOP,
};

const WL_SHM_FORMAT_ARGB8888: u32 = 0;
const KEY_ESC: u32 = 1;
const KEY_BACKSPACE: u32 = 14;
const KEY_ENTER: u32 = 28;
const KEY_UP: u32 = 103;
const KEY_DOWN: u32 = 108;

const BG: u32 = 0xE0181820;
const FG: u32 = 0xFFF0F0F0;
const HI: u32 = 0xFF3D5270;

fn path_index_db() -> PathBuf {
    let config_db = PathBuf::from("/config/strat/path-index.db");
    if config_db.parent().is_some_and(|p| p.exists()) {
        return config_db;
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/strat/path-index.db");
    }
    PathBuf::from("/tmp/strat-path-index.db")
}

fn frecency_db() -> PathBuf {
    let p = PathBuf::from("/config/strat/frecency.db");
    if p.parent().is_some_and(|x| x.exists()) {
        return p;
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/strat/frecency.db");
    }
    PathBuf::from("/tmp/strat-frecency.db")
}

fn record_frecency(path: &Path) {
    let Ok(conn) = Connection::open(frecency_db()) else {
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let p = path.to_string_lossy();
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS path_use (
            path TEXT PRIMARY KEY,
            use_count INTEGER NOT NULL DEFAULT 0,
            last_used_epoch INTEGER NOT NULL DEFAULT 0
        );",
    );
    let _ = conn.execute(
        "INSERT INTO path_use(path, use_count, last_used_epoch)
         VALUES(?1, 1, ?2)
         ON CONFLICT(path) DO UPDATE SET
           use_count = path_use.use_count + 1,
           last_used_epoch = excluded.last_used_epoch",
        params![p.as_ref(), now],
    );
}

fn search_paths(conn: &Connection, q: &str, limit: usize) -> Vec<String> {
    let t = q.trim();
    if t.is_empty() {
        let mut stmt = match conn.prepare(
            "SELECT path FROM path_index ORDER BY CASE kind WHEN 'file' THEN 0 WHEN 'dir' THEN 1 ELSE 2 END, path LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = stmt.query_map(params![limit as i64], |row| row.get::<_, String>(0));
        return rows.map(|r| r.filter_map(|x| x.ok()).collect()).unwrap_or_default();
    }
    let pat = format!("%{}%", t.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let mut stmt = match conn.prepare(
        "SELECT path FROM path_index WHERE path LIKE ?1 ESCAPE '\\' ORDER BY CASE kind WHEN 'file' THEN 0 WHEN 'dir' THEN 1 ELSE 2 END, path LIMIT ?2",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = stmt.query_map(params![pat, limit as i64], |row| row.get::<_, String>(0));
    rows.map(|r| r.filter_map(|x| x.ok()).collect()).unwrap_or_default()
}

fn launch_selected(path: &Path) {
    record_frecency(path);
    if path.is_dir() {
        return;
    }
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return,
    };
    if !meta.is_file() {
        return;
    }
    let mode = meta.permissions().mode();
    let wayland = env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-1".into());
    let runtime = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run".into());
    if mode & 0o111 != 0 {
        let _ = Command::new(path)
            .env("WAYLAND_DISPLAY", &wayland)
            .env("XDG_RUNTIME_DIR", &runtime)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        return;
    }
    if Path::new("/usr/bin/xdg-open").exists() {
        let _ = Command::new("/usr/bin/xdg-open")
            .arg(path)
            .env("WAYLAND_DISPLAY", &wayland)
            .env("XDG_RUNTIME_DIR", &runtime)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

fn fill_rect(buf: &mut [u8], stride: u32, w: i32, h: i32, x: i32, y: i32, rw: i32, rh: i32, color: u32) {
    let b = color.to_le_bytes();
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + rw).min(w);
    let y1 = (y + rh).min(h);
    for py in y0..y1 {
        for px in x0..x1 {
            let o = (py as u32 * stride + px as u32 * 4) as usize;
            if o + 4 <= buf.len() {
                buf[o..o + 4].copy_from_slice(&b);
            }
        }
    }
}

fn draw_text(buf: &mut [u8], stride: u32, w: i32, h: i32, x: i32, y: i32, text: &str, color: u32) {
    const FONT: &[(char, [u8; 7])] = &[
        ('0', [0x3E, 0x51, 0x49, 0x45, 0x3E, 0x00, 0x00]),
        ('1', [0x00, 0x42, 0x7F, 0x40, 0x00, 0x00, 0x00]),
        ('2', [0x42, 0x61, 0x51, 0x49, 0x46, 0x00, 0x00]),
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
        ('/', [0x04, 0x08, 0x10, 0x20, 0x40, 0x00, 0x00]),
        ('.', [0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00]),
        ('-', [0x00, 0x08, 0x7F, 0x08, 0x00, 0x00, 0x00]),
        ('_', [0x00, 0x00, 0x00, 0x00, 0x7F, 0x00, 0x00]),
        ('~', [0x08, 0x34, 0x28, 0x34, 0x08, 0x00, 0x00]),
        (':', [0x00, 0x36, 0x36, 0x00, 0x00, 0x00, 0x00]),
    ];
    let bytes = color.to_le_bytes();
    let mut cx = x;
    for ch in text.chars() {
        if ch == ' ' {
            cx += 6;
            continue;
        }
        let chl = ch.to_ascii_lowercase();
        let glyph = FONT.iter().find(|(c, _)| *c == chl);
        if let Some((_, rows)) = glyph {
            // 5x7 font is column-major: first 5 bytes are columns, bit0 is top row.
            for col_idx in 0..5 {
                let col = rows[col_idx];
                for row_idx in 0..7 {
                    if (col >> row_idx) & 1 == 1 {
                        let px = cx + col_idx as i32;
                        let py = y + row_idx as i32;
                        if px >= 0 && px < w && py >= 0 && py < h {
                            let o = (py as u32 * stride + px as u32 * 4) as usize;
                            if o + 4 <= buf.len() {
                                buf[o..o + 4].copy_from_slice(&bytes);
                            }
                        }
                    }
                }
            }
        }
        cx += 6;
    }
}

fn map_key_to_char(key: u32, shift: bool) -> Option<char> {
    match key {
        2..=11 => {
            let ch = match key {
                11 => '0',
                k => char::from(b'1' + (k - 2) as u8),
            };
            Some(if shift {
                match key {
                    11 => ')',
                    2 => '!',
                    3 => '@',
                    4 => '#',
                    5 => '$',
                    6 => '%',
                    7 => '^',
                    8 => '&',
                    9 => '*',
                    10 => '(',
                    _ => ch,
                }
            } else {
                ch
            })
        }
        16..=25 => Some(match key {
            16 => if shift { 'Q' } else { 'q' },
            17 => if shift { 'W' } else { 'w' },
            18 => if shift { 'E' } else { 'e' },
            19 => if shift { 'R' } else { 'r' },
            20 => if shift { 'T' } else { 't' },
            21 => if shift { 'Y' } else { 'y' },
            22 => if shift { 'U' } else { 'u' },
            23 => if shift { 'I' } else { 'i' },
            24 => if shift { 'O' } else { 'o' },
            25 => if shift { 'P' } else { 'p' },
            _ => return None,
        }),
        30..=38 => Some(match key {
            30 => if shift { 'A' } else { 'a' },
            31 => if shift { 'S' } else { 's' },
            32 => if shift { 'D' } else { 'd' },
            33 => if shift { 'F' } else { 'f' },
            34 => if shift { 'G' } else { 'g' },
            35 => if shift { 'H' } else { 'h' },
            36 => if shift { 'J' } else { 'j' },
            37 => if shift { 'K' } else { 'k' },
            38 => if shift { 'L' } else { 'l' },
            _ => return None,
        }),
        39 => Some(if shift { ':' } else { ';' }),
        44..=50 => Some(match key {
            44 => if shift { 'Z' } else { 'z' },
            45 => if shift { 'X' } else { 'x' },
            46 => if shift { 'C' } else { 'c' },
            47 => if shift { 'V' } else { 'v' },
            48 => if shift { 'B' } else { 'b' },
            49 => if shift { 'N' } else { 'n' },
            50 => if shift { 'M' } else { 'm' },
            _ => return None,
        }),
        12 => Some(if shift { '_' } else { '-' }),
        13 => Some(if shift { '+' } else { '=' }),
        51 => Some(if shift { '<' } else { ',' }),
        52 => Some(if shift { '>' } else { '.' }),
        53 => Some(if shift { '?' } else { '/' }),
        57 => Some(' '),
        _ => None,
    }
}

fn bind_global(
    client: &mut WaylandClient,
    registry_id: u32,
    globals: &std::collections::HashMap<String, (u32, u32)>,
    name: &str,
    ver: u32,
    iface: Interface,
) -> Result<u32, Box<dyn std::error::Error>> {
    let (n, sv) = globals
        .get(name)
        .copied()
        .ok_or_else(|| format!("missing Wayland global {name}"))?;
    let id = client.registry().allocate();
    client.registry().set_interface(id, iface);
    let version = std::cmp::min(sv, ver);
    WlRegistry::new(registry_id).bind(n, name, version, id, client.socket());
    Ok(id)
}

fn bind_layer_shell(
    client: &mut WaylandClient,
    registry_id: u32,
    globals: &std::collections::HashMap<String, (u32, u32)>,
) -> Result<u32, Box<dyn std::error::Error>> {
    let (n, sv) = globals
        .get("zwlr_layer_shell_v1")
        .copied()
        .ok_or("missing zwlr_layer_shell_v1")?;
    let id = client.registry().allocate();
    let version = std::cmp::min(sv, 4u32);
    WlRegistry::new(registry_id).bind(n, "zwlr_layer_shell_v1", version, id, client.socket());
    Ok(id)
}

fn redraw_ui(
    buf: &mut [u8],
    stride: u32,
    width: i32,
    height: i32,
    query: &str,
    results: &[String],
    sel: usize,
) {
    fill_rect(buf, stride, width, height, 0, 0, width, height, BG);
    draw_text(
        buf,
        stride,
        width,
        height,
        12,
        16,
        "Spotlite — filter paths  Enter run  Esc close",
        FG,
    );
    let qline = format!("> {}", query);
    draw_text(buf, stride, width, height, 12, 40, &qline, FG);
    let row_h = 14;
    let max_rows = ((height - 80) / row_h).max(1) as usize;
    let start = sel.saturating_sub(max_rows / 2);
    let mut y = 72i32;
    for idx in start..results.len().min(start + max_rows) {
        let path = &results[idx];
        let line = if path.len() > 110 {
            format!("…{}", &path[path.len() - 109..])
        } else {
            path.clone()
        };
        let col = if idx == sel { HI } else { FG };
        draw_text(buf, stride, width, height, 12, y, &line, col);
        y += row_h;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = path_index_db();
    let conn = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Spotlite: open {}: {e}", db_path.display()))?;

    let mut client = WaylandClient::new()?;
    let registry_id = client.registry().allocate();
    client.registry().set_interface(registry_id, Interface::WlRegistry);
    WlDisplay::new(1).get_registry(registry_id, client.socket());
    let globals_ev = client.roundtrip()?;
    let mut globals: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();
    for ev in globals_ev {
        if let Event::RegistryGlobal { name, interface, version } = ev {
            globals.insert(interface, (name, version));
        }
    }

    let compositor_id = bind_global(&mut client, registry_id, &globals, "wl_compositor", 4, Interface::WlCompositor)?;
    let shm_id = bind_global(&mut client, registry_id, &globals, "wl_shm", 1, Interface::WlShm)?;
    let seat_id = bind_global(&mut client, registry_id, &globals, "wl_seat", 5, Interface::WlSeat)?;
    let layer_shell_id = bind_layer_shell(&mut client, registry_id, &globals)?;

    let surface_id = client.registry().allocate();
    client.registry().set_interface(surface_id, Interface::WlSurface);
    let layer_surface_id = client.registry().allocate();
    client.register_layer_surface(layer_surface_id);
    let keyboard_id = client.registry().allocate();
    client.registry().set_interface(keyboard_id, Interface::WlKeyboard);

    {
        let s = client.socket();
        WlCompositor::new(compositor_id).create_surface(surface_id, s);
        ZwlrLayerShellV1::new(layer_shell_id).get_layer_surface(
            layer_surface_id,
            surface_id,
            0,
            LAYER_OVERLAY,
            "spotlite",
            s,
        );
        let ls = ZwlrLayerSurfaceV1::new(layer_surface_id);
        ls.set_size(0, 0, s);
        ls.set_anchor(ANCHOR_TOP | ANCHOR_BOTTOM | ANCHOR_LEFT | ANCHOR_RIGHT, s);
        ls.set_exclusive_zone(-1, s);
        ls.set_keyboard_interactivity(1, s);
        WlSurface::new(surface_id).commit(s);
        WlSeat::new(seat_id).get_keyboard(keyboard_id, s);
    }

    let ls = ZwlrLayerSurfaceV1::new(layer_surface_id);
    let mut width = 1920i32;
    let mut height = 1080i32;
    'cfg: loop {
        for event in client.poll()? {
            if let Event::LayerSurfaceConfigure { serial, width: w, height: h, .. } = event {
                ls.ack_configure(serial, client.socket());
                if w > 0 {
                    width = w as i32;
                }
                if h > 0 {
                    height = h as i32;
                }
                WlSurface::new(surface_id).commit(client.socket());
                break 'cfg;
            }
        }
    }

    let stride = width * 4;
    let pool_size = (stride * height) as usize;
    let pool = ShmPool::create(pool_size)?;
    let shm_fd = pool.fd();
    let mut shm_buffer = ShmBuffer::new(pool, 0, width as u32, height as u32, stride as u32);

    let pool_id = client.registry().allocate();
    client.registry().set_interface(pool_id, Interface::WlShmPool);
    WlShm::new(shm_id).create_pool(pool_id, shm_fd, pool_size as i32, client.socket());

    let buffer_id = client.registry().allocate();
    client.registry().set_interface(buffer_id, Interface::WlBuffer);
    WlShmPool::new(pool_id).create_buffer(
        buffer_id,
        0,
        width,
        height,
        stride as i32,
        WL_SHM_FORMAT_ARGB8888,
        client.socket(),
    );

    let mut query = String::new();
    let mut results: Vec<String> = search_paths(&conn, "", 48);
    let mut sel: usize = 0;
    let mut shift = false;

    redraw_ui(
        shm_buffer.data_mut(),
        stride as u32,
        width,
        height,
        &query,
        &results,
        sel,
    );
    WlSurface::new(surface_id).attach(buffer_id, 0, 0, client.socket());
    WlSurface::new(surface_id).damage(0, 0, width, height, client.socket());
    WlSurface::new(surface_id).commit(client.socket());

    loop {
        for ev in client.poll()? {
            match ev {
                Event::KeyboardModifiers { mods_depressed, .. } => {
                    shift = (mods_depressed & 0x01) != 0;
                }
                Event::KeyboardKey { key, state, .. } if state == 1 => {
                    if key == KEY_ESC {
                        return Ok(());
                    }
                    if key == KEY_ENTER {
                        if let Some(p) = results.get(sel) {
                            launch_selected(Path::new(p));
                        }
                        return Ok(());
                    }
                    if key == KEY_UP && sel > 0 {
                        sel -= 1;
                    } else if key == KEY_DOWN && sel + 1 < results.len() {
                        sel += 1;
                    } else if key == KEY_BACKSPACE {
                        query.pop();
                        results = search_paths(&conn, &query, 48);
                        sel = sel.min(results.len().saturating_sub(1));
                    } else if let Some(c) = map_key_to_char(key, shift) {
                        if query.len() < 200 {
                            query.push(c);
                            results = search_paths(&conn, &query, 48);
                            sel = 0;
                        }
                    }
                    redraw_ui(
                        shm_buffer.data_mut(),
                        stride as u32,
                        width,
                        height,
                        &query,
                        &results,
                        sel,
                    );
                    WlSurface::new(surface_id).attach(buffer_id, 0, 0, client.socket());
                    WlSurface::new(surface_id).damage(0, 0, width, height, client.socket());
                    WlSurface::new(surface_id).commit(client.socket());
                }
                Event::LayerSurfaceClosed { .. } => return Ok(()),
                _ => {}
            }
        }
    }
}
