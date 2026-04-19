mod font;
mod frecency;
mod file_browser;
mod keyboard;
mod parser;
mod pty;
mod renderer;
mod screen;
mod wayland;

use font::{FONT_HEIGHT, FONT_WIDTH};
use frecency::FrecencyStore;
use file_browser::{FileBrowser, ViewMode};
use keyboard::{is_control_key, keysym_to_char, keysym_to_control};
use nix::poll::{poll, PollFd, PollFlags};
use parser::VtParser;
use pty::Pty;
use renderer::{
    ContentView, Renderer, SplitLayout, TitleBarHit, UiOverlay,
};
use screen::ScreenBuffer;
use rusqlite::{Connection, OpenFlags};
use std::fs;
use std::os::unix::io::BorrowedFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use wayland::WaylandWindow;

const INITIAL_WIDTH: i32 = 800;
const INITIAL_HEIGHT: i32 = 600;
const SCROLLBACK_LINES: usize = 10_000;
const CWD_SYNC_INTERVAL: Duration = Duration::from_millis(500);
const POLL_TIMEOUT_MS: u16 = 100;

const KEYSYM_SHIFT_LEFT: u32 = 0xFFE1;
const KEYSYM_SHIFT_RIGHT: u32 = 0xFFE2;
const KEYSYM_PAGE_UP: u32 = 0xFF55;
const KEYSYM_PAGE_DOWN: u32 = 0xFF56;
const KEYSYM_RIGHT: u32 = 0xFF53;
const KEYSYM_LEFT: u32 = 0xFF51;
const KEYSYM_F6: u32 = 0xFFC3;
const KEYSYM_F7: u32 = 0xFFC4;
const KEYSYM_TAB: u32 = 0xFF09;
const KEYSYM_ENTER: u32 = 0xFF0D;
const KEYSYM_BACKSPACE: u32 = 0xFF08;
const KEYSYM_UP: u32 = 0xFF52;
const KEYSYM_DOWN: u32 = 0xFF54;
const KEYSYM_SPACE: u32 = 0x0020;

const BTN_LEFT: u32 = 272;
const MOUSE_DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(450);
/// `wl_pointer.axis` — vertical scroll (Wayland core protocol).
const WL_POINTER_AXIS_VERTICAL_SCROLL: u32 = 0;

fn sync_shell_cwd(pty: &Pty, last_shell_cwd: &mut Option<String>) -> Option<String> {
    let path = format!("/proc/{}/cwd", pty.child_pid());
    let cwd = fs::read_link(path)
        .ok()
        .map(|value| value.to_string_lossy().to_string());

    if cwd.is_none() || *last_shell_cwd == cwd {
        return None;
    }

    if let Some(current) = cwd.as_ref() {
        let _ = fs::write("/tmp/stratterm-shell-cwd", current);
        std::env::set_var("STRATTERM_SHELL_CWD", current);
    }
    *last_shell_cwd = cwd;
    last_shell_cwd.clone()
}

fn update_status_title(window: &mut WaylandWindow, browser: &FileBrowser, screen: &ScreenBuffer) {
    let mode = match browser.view_mode() {
        ViewMode::Flat => "flat",
        ViewMode::Tree => "tree",
    };
    let title = format!(
        "StratTerm [{mode}] items:{} scroll:{} cwd:{}",
        browser.entries().len(),
        screen.scrollback_offset,
        browser.cwd().display()
    );
    window.set_title(&title);
}

fn trim_for_width(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn default_file_explorer_view(s: &str) -> ViewMode {
    match s.trim().to_ascii_lowercase().as_str() {
        "tree" => ViewMode::Tree,
        _ => ViewMode::Flat,
    }
}

fn shell_single_quote(path: &str) -> String {
    let mut out = String::from("'");
    for ch in path.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn path_is_readable(path: &Path) -> bool {
    fs::OpenOptions::new().read(true).open(path).is_ok()
}

fn path_index_db_path() -> PathBuf {
    let config_db = PathBuf::from("/config/strat/path-index.db");
    if config_db.parent().is_some_and(|parent| parent.exists()) {
        return config_db;
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config/strat/path-index.db");
    }
    PathBuf::from("/tmp/strat-path-index.db")
}

/// Read-only peek at the indexer SQLite DB (same path rules as `stratterm-indexer`).
fn indexer_preview_stub() -> Option<String> {
    let db = path_index_db_path();
    if !db.is_file() {
        return Some("Spotlite index: no path-index.db yet".to_string());
    }
    let conn = match Connection::open_with_flags(
        &db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(_) => return Some("Spotlite index: cannot open DB read-only".to_string()),
    };
    let count: i64 = match conn.query_row("SELECT COUNT(*) FROM path_index", [], |row| row.get(0))
    {
        Ok(c) => c,
        Err(_) => return Some("Spotlite index: DB present but not readable".to_string()),
    };
    let max_epoch: Option<i64> = conn
        .query_row("SELECT MAX(indexed_epoch) FROM path_index", [], |row| row.get(0))
        .ok()
        .flatten();
    let detail = match max_epoch {
        Some(epoch) if count > 0 => format!("last batch epoch {epoch}"),
        _ => "no rows".to_string(),
    };
    Some(format!("Spotlite: {count} paths ({detail})"))
}

/// Slice of entries to draw in the file list, sized to match the renderer row cap (`list_band.row_count`).
fn browser_visible_range(selected: usize, len: usize, max_visible: usize) -> (usize, usize) {
    if len == 0 {
        return (0, 0);
    }
    let w = max_visible.max(1).min(len).min(32);
    let mut start = selected.saturating_sub(w / 2);
    if start + w > len {
        start = len.saturating_sub(w);
    }
    let end = start + w;
    (start, end)
}

fn build_overlay(
    browser: &FileBrowser,
    screen: &ScreenBuffer,
    selected_index: usize,
    browser_message: &str,
    ghost_suffix: &str,
    list_row_cap: usize,
    status_bar_enabled: bool,
) -> UiOverlay {
    let mode = match browser.view_mode() {
        ViewMode::Flat => "flat",
        ViewMode::Tree => "tree",
    };
    let status_chip = if status_bar_enabled {
        format!(
            "mode:{mode} items:{} scroll:{} cwd:{}",
            browser.entries().len(),
            screen.scrollback_offset,
            browser.cwd().display()
        )
    } else {
        String::new()
    };
    let mut overlay = UiOverlay {
        status_chip,
        browser_title: format!(
            "Browser [{}] ({})",
            mode,
            trim_for_width(&browser.cwd().display().to_string(), 48)
        ),
        browser_lines: Vec::new(),
        preview_title: "Preview".to_string(),
        preview_lines: Vec::new(),
        ghost_suffix: ghost_suffix.to_string(),
    };

    let entries = browser.entries();
    if entries.is_empty() {
        overlay.browser_lines.push("(no entries)".to_string());
        overlay.preview_lines.push(trim_for_width(browser_message, 44));
        if let Some(err) = browser.list_dir_error() {
            overlay
                .preview_lines
                .push(trim_for_width(&format!("List: {}", err), 44));
        }
        if let Some(line) = indexer_preview_stub() {
            overlay.preview_lines.push(trim_for_width(&line, 44));
        }
        return overlay;
    }

    let selected = selected_index.min(entries.len().saturating_sub(1));
    let (start, end) = browser_visible_range(selected, entries.len(), list_row_cap);
    for (index, entry) in entries.iter().enumerate().take(end).skip(start) {
        let marker = if index == selected { ">" } else { " " };
        let indent = " ".repeat(entry.depth * 2);
        let dir_tag = if entry.is_dir { "/" } else { "" };
        let label = if entry.is_parent_row {
            entry.name.clone()
        } else {
            format!("{indent}{}{}", entry.name, dir_tag)
        };
        overlay
            .browser_lines
            .push(format!("{marker} {}", trim_for_width(&label, 72)));
    }

    let selected_entry = &entries[selected];
    let preview = browser.preview_for(&selected_entry.path);
    overlay.preview_lines.push(trim_for_width(browser_message, 44));
    if let Some(err) = browser.list_dir_error() {
        overlay
            .preview_lines
            .push(trim_for_width(&format!("List: {}", err), 44));
    }
    if let Some(line) = indexer_preview_stub() {
        overlay.preview_lines.push(trim_for_width(&line, 44));
    }
    match preview {
        file_browser::PreviewKind::FolderSummary(text)
        | file_browser::PreviewKind::ScriptHint(text)
        | file_browser::PreviewKind::ConfigSummary(text)
        | file_browser::PreviewKind::BinaryHint(text) => {
            overlay.preview_lines.push(trim_for_width(&text, 44));
        }
        file_browser::PreviewKind::TextSnippet(text) => {
            for line in text.lines().take(8) {
                overlay.preview_lines.push(trim_for_width(line, 44));
            }
        }
    }

    overlay
}

/// True when the pointer is over the **file list rows** only (not preview, not title bar).
fn pointer_in_filesystem_list(pointer_x: i32, pointer_y: i32, layout: &SplitLayout) -> bool {
    if pointer_y < 0 || pointer_y >= layout.buffer_height as i32 {
        return false;
    }
    let bw = layout.buffer_width as i32;
    if pointer_x < 0 || pointer_x >= bw {
        return false;
    }
    let band = layout.list_band();
    let top = band.list_top as i32;
    let bottom = band.bottom_exclusive() as i32;
    pointer_y >= top && pointer_y < bottom
}

fn entry_index_from_pointer(
    pointer_x: i32,
    pointer_y: i32,
    layout: &SplitLayout,
    selected: usize,
    entry_count: usize,
) -> Option<usize> {
    if entry_count == 0 {
        return None;
    }
    let band = layout.list_band();
    let list_start_y = band.list_top as i32;
    let bottom = band.bottom_exclusive() as i32;
    let bw = layout.buffer_width as i32;
    if pointer_x < 0 || pointer_x >= bw || pointer_y < list_start_y || pointer_y >= bottom {
        return None;
    }
    let row = ((pointer_y - list_start_y) / FONT_HEIGHT as i32) as usize;
    let max_rows = band.row_count as usize;
    if row >= max_rows {
        return None;
    }

    let (start, end) = browser_visible_range(selected, entry_count, band.row_count as usize);
    let index = start + row;
    if index >= end || index >= entry_count {
        None
    } else {
        Some(index)
    }
}

fn activate_browser_entry(
    pty: &Pty,
    browser: &mut FileBrowser,
    browser_selected: &mut usize,
    browser_message: &mut String,
    pending_script_path: &mut Option<PathBuf>,
    pending_script_ts: &mut Option<Instant>,
) -> Result<bool, String> {
    if browser.entries().is_empty() {
        return Ok(false);
    }

    let selected = browser.entries()[*browser_selected].clone();
    if selected.is_dir {
        let path_disp = selected.path.display().to_string();
        match browser.navigate_to(selected.path.clone()) {
            Ok(()) => {
                *browser_selected = 0;
                *pending_script_path = None;
                *pending_script_ts = None;
                *browser_message = format!("Opened {}", path_disp);
                return Ok(true);
            }
            Err(reason) => {
                *browser_message = format!("Cannot open directory: {}", reason);
                return Ok(true);
            }
        }
    }

    match browser.action_for_double_click(&selected.path) {
        file_browser::DoubleClickAction::RunScriptConfirm => {
            if !path_is_readable(&selected.path) {
                *browser_message = format!("Cannot read {}", selected.path.display());
                *pending_script_path = None;
                *pending_script_ts = None;
                return Ok(true);
            }
            let now = Instant::now();
            let double_confirmed = pending_script_path
                .as_ref()
                .is_some_and(|path| path == &selected.path)
                && pending_script_ts
                    .is_some_and(|ts| now.duration_since(ts) <= Duration::from_secs(2));
            if double_confirmed {
                let q = shell_single_quote(&selected.path.to_string_lossy());
                let command = format!("sh {q}\n");
                pty.write(command.as_bytes())
                    .map_err(|e| format!("PTY write failed: {}", e))?;
                *browser_message = format!("Ran script {}", selected.path.display());
                *pending_script_path = None;
                *pending_script_ts = None;
            } else {
                *browser_message = format!("Press Enter again to run {}", selected.name);
                *pending_script_path = Some(selected.path.clone());
                *pending_script_ts = Some(now);
            }
        }
        file_browser::DoubleClickAction::OpenConfigEditor => {
            if !path_is_readable(&selected.path) {
                *browser_message = format!("Cannot read {}", selected.path.display());
                return Ok(true);
            }
            let q = shell_single_quote(&selected.path.to_string_lossy());
            let command = format!("nano {q} || vi {q}\n");
            pty.write(command.as_bytes())
                .map_err(|e| format!("PTY write failed: {}", e))?;
            *browser_message = format!("Editing {}", selected.path.display());
        }
        file_browser::DoubleClickAction::OpenWithXdg => {
            if !path_is_readable(&selected.path) {
                *browser_message = format!("Cannot read {}", selected.path.display());
                return Ok(true);
            }
            let q = shell_single_quote(&selected.path.to_string_lossy());
            let command = format!("xdg-open {q} >/dev/null 2>&1 &\n");
            pty.write(command.as_bytes())
                .map_err(|e| format!("PTY write failed: {}", e))?;
            *browser_message = format!("Opened {}", selected.path.display());
        }
        file_browser::DoubleClickAction::RefuseExecutableAutoOpen => {
            *browser_message = format!(
                "Not opening executable `{}` from browser; use the shell.",
                selected.name
            );
        }
        file_browser::DoubleClickAction::NavigateDirectory => {}
    }
    Ok(true)
}

fn track_typed_line(typed_line: &mut String, key: u32, state: u32, ch: Option<char>) {
    if state != 1 {
        return;
    }
    match key {
        KEYSYM_ENTER => typed_line.clear(),
        KEYSYM_BACKSPACE => {
            let _ = typed_line.pop();
        }
        _ => {
            if let Some(value) = ch {
                typed_line.push(value);
            }
        }
    }
}

fn suggest_cd_completion(typed_line: &str, frecency: Option<&FrecencyStore>) -> Option<String> {
    let store = frecency?;
    if typed_line.trim_start().starts_with("cd -s ") {
        let shortcut = typed_line.trim_start().strip_prefix("cd -s ")?.trim();
        let expanded = store.expand_cd_shortcut(shortcut)?;
        let expanded_str = expanded.to_string_lossy().to_string();
        if expanded_str.starts_with(shortcut) {
            return Some(expanded_str[shortcut.len()..].to_string());
        }
        return None;
    }
    store.best_completion_for_cd(typed_line)
}

fn refresh_ghost_suffix(typed_line: &str, frecency: Option<&FrecencyStore>) -> String {
    suggest_cd_completion(typed_line, frecency).unwrap_or_default()
}

fn maybe_record_cd_use(typed_line: &str, frecency: Option<&FrecencyStore>) {
    let store = match frecency {
        Some(value) => value,
        None => return,
    };
    let trimmed = typed_line.trim();
    let path = if let Some(value) = trimmed.strip_prefix("cd ") {
        value.trim()
    } else {
        return;
    };
    if path.is_empty() || path.starts_with('-') {
        return;
    }
    store.record_use(PathBuf::from(path).as_path());
}

fn render_frame(
    renderer: &mut Renderer,
    screen: &ScreenBuffer,
    window: &mut WaylandWindow,
    browser: &FileBrowser,
    width: i32,
    height: i32,
    cols: usize,
    focus: ContentView,
    browser_selected: usize,
    browser_message: &str,
    ghost_suffix: &str,
    client_title_bar: bool,
    status_bar_enabled: bool,
) -> Result<(), String> {
    let layout = SplitLayout::compute(width, height, cols, client_title_bar);
    let list_row_cap = layout.list_band().row_count as usize;
    renderer.render(
        screen,
        window,
        Some(&build_overlay(
            browser,
            screen,
            browser_selected,
            browser_message,
            ghost_suffix,
            list_row_cap,
            status_bar_enabled,
        )),
        &layout,
        focus,
    )
}

fn main() -> Result<(), String> {
    let strat_cfg = stratsettings::StratSettings::load().unwrap_or_default();
    let client_title_bar = strat_cfg.stratterm.file_explorer.client_title_bar_enabled;
    let status_bar_enabled = strat_cfg.stratterm.file_explorer.status_bar_enabled;

    let mut window = WaylandWindow::new(INITIAL_WIDTH, INITIAL_HEIGHT)
        .map_err(|e| format!("Failed to create Wayland window: {}", e))?;

    let (mut width, mut height) = window.get_size();
    let mut cols = (width as usize) / FONT_WIDTH;
    let mut layout = SplitLayout::compute(width, height, cols, client_title_bar);
    let mut rows = layout.terminal_rows.max(1) as usize;

    let mut screen = ScreenBuffer::new(rows, cols);
    let scrollback_cap = {
        let n = strat_cfg.stratterm.term.scrollback_max_lines;
        if n > 0 {
            n
        } else {
            SCROLLBACK_LINES
        }
    };
    screen.set_scrollback_max(scrollback_cap);
    let pty = Pty::new(rows as u16, cols as u16)
        .map_err(|e| format!("Failed to create PTY: {}", e))?;
    let frecency = FrecencyStore::open_default().ok();
    let explorer_view = default_file_explorer_view(&strat_cfg.stratterm.file_explorer.default_view);
    let mut file_browser =
        FileBrowser::with_view_mode(PathBuf::from("/home"), explorer_view);
    let mut parser = VtParser::new();
    let mut renderer = Renderer::new(width as u32, height as u32);

    let mut typed_line = String::new();
    let mut ghost_suffix = refresh_ghost_suffix("", frecency.as_ref());
    let mut focus = ContentView::Terminal;
    let mut browser_selected = 0usize;
    let mut browser_message =
        String::from("Explorer above, shell below  |  F7 or title bar switches focus");
    let mut pending_script_path: Option<PathBuf> = None;
    let mut pending_script_ts: Option<Instant> = None;
    let mut pointer_x = 0i32;
    let mut pointer_y = 0i32;
    let mut last_click_entry: Option<usize> = None;
    let mut last_click_ts: Option<Instant> = None;

    render_frame(
        &mut renderer,
        &screen,
        &mut window,
        &file_browser,
        width,
        height,
        cols,
        focus,
        browser_selected,
        &browser_message,
        &ghost_suffix,
        client_title_bar,
        status_bar_enabled,
    )
    .map_err(|e| format!("Initial render failed: {}", e))?;
    update_status_title(&mut window, &file_browser, &screen);

    let pty_fd = pty.raw_fd();
    let wayland_fd = window.raw_fd();
    let mut shift_pressed = false;
    let mut last_cwd_sync = Instant::now();
    let mut last_shell_cwd: Option<String> = None;
    let mut buf = [0u8; 8192];

    loop {
        let mut poll_fds = unsafe {
            [
                PollFd::new(BorrowedFd::borrow_raw(pty_fd), PollFlags::POLLIN),
                PollFd::new(BorrowedFd::borrow_raw(wayland_fd), PollFlags::POLLIN),
            ]
        };
        let n = poll(&mut poll_fds, POLL_TIMEOUT_MS).map_err(|e| format!("poll failed: {}", e))?;

        if last_cwd_sync.elapsed() >= CWD_SYNC_INTERVAL {
            if let Some(cwd) = sync_shell_cwd(&pty, &mut last_shell_cwd) {
                let _ = file_browser.navigate_to(PathBuf::from(cwd));
                update_status_title(&mut window, &file_browser, &screen);
            }
            last_cwd_sync = Instant::now();
        }
        if n == 0 {
            continue;
        }

        let pty_ready = poll_fds[0]
            .revents()
            .map(|r| r.contains(PollFlags::POLLIN))
            .unwrap_or(false);
        let wayland_ready = poll_fds[1]
            .revents()
            .map(|r| r.contains(PollFlags::POLLIN))
            .unwrap_or(false);

        if pty_ready {
            let read = pty
                .read(&mut buf)
                .map_err(|e| format!("PTY read failed: {}", e))?;
            if read == 0 {
                break;
            }
            parser.parse(&mut screen, &buf[..read]);
            render_frame(
                &mut renderer,
                &screen,
                &mut window,
                &file_browser,
                width,
                height,
                cols,
                focus,
                browser_selected,
                &browser_message,
                &ghost_suffix,
                client_title_bar,
                status_bar_enabled,
            )
            .map_err(|e| format!("Render failed: {}", e))?;
            update_status_title(&mut window, &file_browser, &screen);
        }

        if wayland_ready {
            let events = window
                .poll_events()
                .map_err(|e| format!("Wayland poll failed: {}", e))?;
            let mut ui_dirty = false;
            layout = SplitLayout::compute(width, height, cols, client_title_bar);

            for event in events {
                match event {
                    stratlayer::Event::PointerMotion { surface_x, surface_y } => {
                        pointer_x = surface_x as i32;
                        pointer_y = surface_y as i32;
                    }
                    stratlayer::Event::PointerEnter { surface_x, surface_y } => {
                        pointer_x = surface_x as i32;
                        pointer_y = surface_y as i32;
                    }
                    stratlayer::Event::PointerLeave => {
                        last_click_entry = None;
                        last_click_ts = None;
                    }
                    stratlayer::Event::PointerAxis { axis, value } => {
                        if Renderer::title_bar_pick(
                            pointer_x,
                            pointer_y,
                            layout.buffer_width,
                            layout.title_bar_h,
                        )
                            .is_some()
                        {
                            continue;
                        }
                        if focus == ContentView::Filesystem
                            && axis == WL_POINTER_AXIS_VERTICAL_SCROLL
                            && pointer_in_filesystem_list(pointer_x, pointer_y, &layout)
                        {
                            let entry_count = file_browser.entries().len();
                            if entry_count > 0 {
                                let direction = if value > 0.0 {
                                    1i32
                                } else if value < 0.0 {
                                    -1
                                } else {
                                    0
                                };
                                if direction != 0 {
                                    let steps = ((value.abs() / 40.0).ceil() as usize).clamp(1, 6);
                                    let delta = direction * steps as i32;
                                    let next = browser_selected as i32 + delta;
                                    browser_selected = next.clamp(0, (entry_count - 1) as i32) as usize;
                                    browser_message = format!(
                                        "Selected {}",
                                        file_browser.entries()[browser_selected].name
                                    );
                                    ui_dirty = true;
                                }
                            }
                        }
                    }
                    stratlayer::Event::PointerButton { button, state } => {
                        if state == 1 && button == BTN_LEFT {
                            if let Some(hit) = Renderer::title_bar_pick(
                                pointer_x,
                                pointer_y,
                                layout.buffer_width,
                                layout.title_bar_h,
                            ) {
                                match hit {
                                    TitleBarHit::FilesTab => {
                                        focus = ContentView::Filesystem;
                                        browser_message =
                                            "Focus: Files (F7 or Terminal tab for shell)".to_string();
                                        ui_dirty = true;
                                    }
                                    TitleBarHit::TerminalTab => {
                                        focus = ContentView::Terminal;
                                        browser_message =
                                            "Focus: Terminal (F7 or Files tab for explorer)".to_string();
                                        ui_dirty = true;
                                    }
                                    TitleBarHit::Close => {
                                        // Match EOF exit path: wait for shell; Wayland releases on process teardown.
                                        let _ = pty.wait();
                                        return Ok(());
                                    }
                                }
                                continue;
                            }
                            let py = pointer_y as u32;
                            if py >= layout.terminal_top {
                                focus = ContentView::Terminal;
                                ui_dirty = true;
                            } else if py >= layout.title_bar_h && py < layout.separator_y {
                                focus = ContentView::Filesystem;
                                ui_dirty = true;
                            }
                        }
                        if state == 1 && button == BTN_LEFT {
                            if let Some(index) = entry_index_from_pointer(
                                pointer_x,
                                pointer_y,
                                &layout,
                                browser_selected,
                                file_browser.entries().len(),
                            ) {
                                let now = Instant::now();
                                let is_double = last_click_entry == Some(index)
                                    && last_click_ts
                                        .is_some_and(|ts| now.duration_since(ts) <= MOUSE_DOUBLE_CLICK_WINDOW);
                                browser_selected = index;
                                browser_message =
                                    format!("Selected {}", file_browser.entries()[browser_selected].name);
                                if is_double {
                                    let changed = activate_browser_entry(
                                        &pty,
                                        &mut file_browser,
                                        &mut browser_selected,
                                        &mut browser_message,
                                        &mut pending_script_path,
                                        &mut pending_script_ts,
                                    )?;
                                    if changed {
                                        last_click_entry = None;
                                        last_click_ts = None;
                                    }
                                } else {
                                    last_click_entry = Some(index);
                                    last_click_ts = Some(now);
                                }
                                ui_dirty = true;
                            }
                        }
                    }
                    stratlayer::Event::KeyboardKey { key, state, .. } => {
                        if key == KEYSYM_SHIFT_LEFT || key == KEYSYM_SHIFT_RIGHT {
                            shift_pressed = state == 1;
                            continue;
                        }
                        if state != 1 {
                            continue;
                        }

                        if key == KEYSYM_F7 {
                            focus = match focus {
                                ContentView::Terminal => ContentView::Filesystem,
                                ContentView::Filesystem => ContentView::Terminal,
                            };
                            browser_message = match focus {
                                ContentView::Filesystem => {
                                    "Focus: Files — F7 or Terminal tab for shell".to_string()
                                }
                                ContentView::Terminal => {
                                    "Focus: Terminal — F7 or Files tab for explorer".to_string()
                                }
                            };
                            ui_dirty = true;
                            continue;
                        }

                        if focus == ContentView::Filesystem {
                            let entry_count = file_browser.entries().len();
                            if entry_count > 0 {
                                if key == KEYSYM_UP {
                                    browser_selected = browser_selected.saturating_sub(1);
                                    ui_dirty = true;
                                    continue;
                                }
                                if key == KEYSYM_DOWN {
                                    browser_selected = (browser_selected + 1).min(entry_count - 1);
                                    ui_dirty = true;
                                    continue;
                                }
                                if key == KEYSYM_LEFT {
                                    match file_browser.go_up() {
                                        Ok(()) => {
                                            browser_selected = 0;
                                            pending_script_path = None;
                                            pending_script_ts = None;
                                            browser_message =
                                                format!("Up: {}", file_browser.cwd().display());
                                        }
                                        Err(reason) => {
                                            browser_message = format!("Go up: {}", reason);
                                        }
                                    }
                                    ui_dirty = true;
                                    continue;
                                }
                                if key == KEYSYM_SPACE {
                                    let selected = file_browser.entries()[browser_selected].clone();
                                    if selected.is_dir && !selected.is_parent_row {
                                        file_browser.toggle_expand(&selected.path);
                                    }
                                    ui_dirty = true;
                                    continue;
                                }
                                if key == KEYSYM_ENTER {
                                    let _ = activate_browser_entry(
                                        &pty,
                                        &mut file_browser,
                                        &mut browser_selected,
                                        &mut browser_message,
                                        &mut pending_script_path,
                                        &mut pending_script_ts,
                                    )?;
                                    ui_dirty = true;
                                    continue;
                                }
                            }
                            if key == KEYSYM_F6 {
                                file_browser.toggle_view_mode();
                                browser_selected = 0;
                                ui_dirty = true;
                                continue;
                            }
                            continue;
                        }

                        if focus != ContentView::Terminal {
                            continue;
                        }

                        if shift_pressed && key == KEYSYM_PAGE_UP {
                            screen.scrollback_page_up(rows);
                            ui_dirty = true;
                            continue;
                        }
                        if shift_pressed && key == KEYSYM_PAGE_DOWN {
                            screen.scrollback_page_down(rows);
                            ui_dirty = true;
                            continue;
                        }
                        if screen.is_scrollback_active() {
                            screen.reset_scrollback();
                        }

                        if (key == KEYSYM_TAB || key == KEYSYM_RIGHT) && !ghost_suffix.is_empty() {
                            pty.write(ghost_suffix.as_bytes())
                                .map_err(|e| format!("PTY write failed: {}", e))?;
                            typed_line.push_str(&ghost_suffix);
                            ghost_suffix.clear();
                            ui_dirty = true;
                            continue;
                        }

                        if key == KEYSYM_ENTER {
                            maybe_record_cd_use(&typed_line, frecency.as_ref());
                            ghost_suffix.clear();
                        }

                        if is_control_key(key) {
                            if let Some(seq) = keysym_to_control(key) {
                                if !seq.is_empty() {
                                    pty.write(&seq)
                                        .map_err(|e| format!("PTY write failed: {}", e))?;
                                }
                            }
                            track_typed_line(&mut typed_line, key, state, None);
                        } else if let Some(ch) = keysym_to_char(key) {
                            let mut utf = [0u8; 4];
                            let s = ch.encode_utf8(&mut utf);
                            pty.write(s.as_bytes())
                                .map_err(|e| format!("PTY write failed: {}", e))?;
                            track_typed_line(&mut typed_line, key, state, Some(ch));
                        }
                        ghost_suffix = refresh_ghost_suffix(&typed_line, frecency.as_ref());
                        ui_dirty = true;
                    }
                    _ => {}
                }
            }

            if ui_dirty {
                render_frame(
                    &mut renderer,
                    &screen,
                    &mut window,
                    &file_browser,
                    width,
                    height,
                    cols,
                    focus,
                    browser_selected,
                    &browser_message,
                    &ghost_suffix,
                    client_title_bar,
                    status_bar_enabled,
                )
                .map_err(|e| format!("Render failed: {}", e))?;
                update_status_title(&mut window, &file_browser, &screen);
            }

            if let Some((new_w, new_h)) = window.commit_pending_size() {
                if new_w != width || new_h != height {
                    width = new_w;
                    height = new_h;
                    cols = (width as usize) / FONT_WIDTH;
                    layout = SplitLayout::compute(width, height, cols, client_title_bar);
                    rows = layout.terminal_rows.max(1) as usize;
                    screen.resize(rows, cols);
                    renderer.resize(width as u32, height as u32);
                    pty.resize(rows as u16, cols as u16)
                        .map_err(|e| format!("PTY resize failed: {}", e))?;
                    render_frame(
                        &mut renderer,
                        &screen,
                        &mut window,
                        &file_browser,
                        width,
                        height,
                        cols,
                        focus,
                        browser_selected,
                        &browser_message,
                        &ghost_suffix,
                        client_title_bar,
                        status_bar_enabled,
                    )
                    .map_err(|e| format!("Render failed: {}", e))?;
                    update_status_title(&mut window, &file_browser, &screen);
                }
            }
        }
    }

    // Shell exited (PTY read returned 0); same teardown expectations as title-bar close.
    let _ = pty.wait();
    Ok(())
}
