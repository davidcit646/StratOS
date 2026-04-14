use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    gio, Application, ApplicationWindow, Box as GtkBox, Button, Entry, EventControllerKey, Label,
    ListBox, ListBoxRow, Orientation, PolicyType, ScrolledWindow, SelectionMode, Separator,
};
use rusqlite::{params, Connection};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use stratterm::index_settings::{
    indexer_is_disabled, load_index_settings, path_allowed_for_indexing, IndexSettings,
};
use vte4::prelude::*;

const APP_ID: &str = "org.stratos.StratTerm";
const SCROLLBACK_LINES: i64 = 10_000;
const MAX_PREVIEW_LINES: usize = 10;
const MAX_TREE_DEPTH: usize = 2;
const INDEX_MAX_QUEUE: usize = 250_000;
const INDEX_CLOSE_BATCH_LIMIT: usize = 2_000;

#[derive(Clone, Debug, PartialEq, Eq)]
enum EntryKind {
    Up,
    Directory,
    File,
}

#[derive(Clone, Debug)]
struct BrowserEntry {
    path: PathBuf,
    kind: EntryKind,
    depth: usize,
    expanded: bool,
    tree_hint: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateSource {
    Frecency,
    Cwd,
    Home,
    System,
}

#[derive(Clone, Debug, Default)]
struct FrecencyRecord {
    count: u64,
    last_visit: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PathFingerprint {
    is_dir: i64,
    size: i64,
    mtime: i64,
    mode: i64,
}

#[derive(Clone, Debug)]
struct CdCandidate {
    path: PathBuf,
    source: CandidateSource,
    score: u64,
    count: u64,
    last_visit: u64,
}

#[derive(Debug)]
struct AppState {
    current_dir: PathBuf,
    entries: Vec<BrowserEntry>,
    child_pid: Option<glib::Pid>,
    tree_mode: bool,
    frecency_db_path: PathBuf,
    frecency: HashMap<String, FrecencyRecord>,
    command_history: Vec<String>,
    current_ghost: Option<String>,
    ghost_dismissed_for: Option<String>,
    home_dir: Option<PathBuf>,
    system_path_dirs: Vec<PathBuf>,
    advanced_mode: bool,
    expanded_dirs: HashSet<String>,
    pending_script_confirmation: Option<String>,
    index_db_path: PathBuf,
    index_queue: VecDeque<PathBuf>,
    index_seen: HashSet<String>,
    index_in_progress: bool,
    index_last_error: Option<String>,
    index_paused_high_usage: bool,
    startup_ts: u64,
    last_activity_ts: u64,
    index_force_until_ts: u64,
    index_settings: IndexSettings,
    indexing_enabled: bool,
}

#[derive(Clone)]
struct Ui {
    working_label: Label,
    status_label: Label,
    breadcrumb_bar: GtkBox,
    file_list: ListBox,
    preview_label: Label,
    tree_toggle_btn: Button,
    expand_toggle_btn: Button,
    command_entry: Entry,
    ghost_label: Label,
    mode_label: Label,
    terminal: vte4::Terminal,
}

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn apply_modern_styles() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        r#"
window.stratterm-window {
    background-image: linear-gradient(140deg, #edf2f9, #f6f9fd 45%, #e8eef6);
}

.app-shell {
    font-family: "IBM Plex Sans", "Noto Sans", "Cantarell", sans-serif;
}

.title-main {
    font-size: 30px;
    font-weight: 700;
    color: #10243d;
    letter-spacing: 0.2px;
}

.title-sub {
    font-size: 13px;
    font-weight: 600;
    color: #4a617c;
}

.working-path {
    font-size: 14px;
    font-weight: 700;
    color: #143552;
}

.status-chip {
    background: rgba(28, 71, 109, 0.1);
    border-radius: 999px;
    padding: 4px 12px;
    color: #1c456d;
    font-size: 11px;
    font-weight: 700;
}

.section-title {
    font-size: 11px;
    letter-spacing: 0.8px;
    font-weight: 700;
    color: #4a5f78;
}

.surface {
    background: rgba(255, 255, 255, 0.85);
    border-radius: 16px;
    border: 1px solid rgba(82, 113, 147, 0.24);
    padding: 8px;
    box-shadow: 0 8px 24px rgba(19, 45, 74, 0.08);
}

.surface-tight {
    background: rgba(255, 255, 255, 0.92);
    border-radius: 13px;
    border: 1px solid rgba(82, 113, 147, 0.24);
    padding: 6px;
}

.preview-text {
    color: #1d4062;
    font-size: 12px;
}

.terminal-surface {
    background: #111d2f;
    border-radius: 15px;
    border: 1px solid rgba(142, 171, 205, 0.36);
    padding: 6px;
}

.action-row button {
    border-radius: 999px;
    padding: 7px 14px;
    background: #1f4f81;
    color: #f7fbff;
    border: none;
    font-weight: 700;
    box-shadow: 0 4px 10px rgba(19, 52, 86, 0.28);
}

.action-row button:hover {
    background: #2a6299;
}

.tree-btn {
    background: #0f6f66;
}

.tree-btn:hover {
    background: #1a8b80;
}

.entry-row {
    border-radius: 10px;
    margin: 2px 4px;
}

.entry-row:selected {
    background: rgba(31, 82, 130, 0.18);
}

.entry-up {
    background: rgba(148, 169, 194, 0.13);
}

.entry-dir {
    background: rgba(75, 126, 80, 0.09);
}

.entry-file {
    background: rgba(62, 94, 124, 0.05);
}

.entry-label {
    font-family: "JetBrains Mono", "Fira Mono", "Noto Sans Mono", monospace;
    font-size: 12px;
    color: #133552;
}

.prompt-bar {
    background: rgba(255, 255, 255, 0.97);
    border-radius: 14px;
    border: 1px solid rgba(82, 113, 147, 0.26);
    padding: 10px;
}

.prompt-symbol {
    font-family: "JetBrains Mono", "Fira Mono", "Noto Sans Mono", monospace;
    font-size: 16px;
    font-weight: 700;
    color: #1a456f;
}

.prompt-entry {
    font-family: "JetBrains Mono", "Fira Mono", "Noto Sans Mono", monospace;
    font-size: 13px;
}

.ghost-text {
    font-family: "JetBrains Mono", "Fira Mono", "Noto Sans Mono", monospace;
    font-size: 13px;
    color: #758fa9;
}

.mode-chip {
    background: rgba(21, 62, 98, 0.1);
    color: #163f65;
    border-radius: 999px;
    padding: 4px 11px;
    font-size: 11px;
    font-weight: 700;
}
"#,
    );

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_ui(app: &Application) {
    apply_modern_styles();

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let home_dir = env::var_os("HOME").map(PathBuf::from);
    let frecency_db_path = frecency_db_path(home_dir.as_deref());
    let frecency = load_frecency(&frecency_db_path);
    let system_path_dirs = collect_system_path_dirs();
    let index_db_path = index_db_path(home_dir.as_deref());
    let index_settings = load_index_settings(home_dir.as_deref());
    let indexing_enabled =
        !indexer_is_disabled(home_dir.as_deref()) && index_settings.enabled && index_settings.ui_enabled;

    let now = unix_now();
    let state = Rc::new(RefCell::new(AppState {
        current_dir: normalize_dir(cwd),
        entries: Vec::new(),
        child_pid: None,
        tree_mode: false,
        frecency_db_path,
        frecency,
        command_history: Vec::new(),
        current_ghost: None,
        ghost_dismissed_for: None,
        home_dir,
        system_path_dirs,
        advanced_mode: false,
        expanded_dirs: HashSet::new(),
        pending_script_confirmation: None,
        index_db_path,
        index_queue: VecDeque::new(),
        index_seen: HashSet::new(),
        index_in_progress: false,
        index_last_error: None,
        index_paused_high_usage: false,
        startup_ts: now,
        last_activity_ts: now,
        index_force_until_ts: 0,
        index_settings,
        indexing_enabled,
    }));

    {
        let mut app_state = state.borrow_mut();
        let current = app_state.current_dir.clone();
        app_state
            .expanded_dirs
            .insert(current.to_string_lossy().to_string());
        record_directory_visit(&mut app_state, &current);
    }

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Strat Terminal")
        .default_width(1200)
        .default_height(800)
        .build();
    window.add_css_class("stratterm-window");

    let root = GtkBox::new(Orientation::Vertical, 8);
    root.add_css_class("app-shell");
    root.set_margin_top(10);
    root.set_margin_bottom(10);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let title = Label::new(Some("StratOS Command Line"));
    title.add_css_class("title-main");
    title.set_xalign(0.0);
    let subtitle = Label::new(Some("Terminal + file workspace"));
    subtitle.add_css_class("title-sub");
    subtitle.set_xalign(0.0);
    root.append(&title);
    root.append(&subtitle);
    let hero_rule = Separator::new(Orientation::Horizontal);
    hero_rule.set_margin_top(4);
    hero_rule.set_margin_bottom(4);
    root.append(&hero_rule);

    let working_label = Label::new(None);
    working_label.add_css_class("working-path");
    working_label.set_xalign(0.0);

    let status_label = Label::new(Some("Status: starting..."));
    status_label.add_css_class("status-chip");
    status_label.set_xalign(1.0);

    let info_row = GtkBox::new(Orientation::Horizontal, 10);
    info_row.append(&working_label);
    info_row.append(&status_label);
    root.append(&info_row);

    let crumb_title = Label::new(Some("BREADCRUMB"));
    crumb_title.add_css_class("section-title");
    crumb_title.set_xalign(0.0);
    root.append(&crumb_title);

    let breadcrumb_bar = GtkBox::new(Orientation::Horizontal, 6);
    let breadcrumb_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Never)
        .child(&breadcrumb_bar)
        .min_content_height(44)
        .build();
    breadcrumb_scroll.add_css_class("surface-tight");
    root.append(&breadcrumb_scroll);

    let body = GtkBox::new(Orientation::Vertical, 8);
    body.set_vexpand(true);

    let files_title = Label::new(Some("FILES"));
    files_title.add_css_class("section-title");
    files_title.set_xalign(0.0);
    body.append(&files_title);

    let file_list = ListBox::new();
    file_list.set_activate_on_single_click(false);
    file_list.set_selection_mode(SelectionMode::Single);

    let file_scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&file_list)
        .build();
    file_scroller.add_css_class("surface");
    body.append(&file_scroller);

    let preview_title = Label::new(Some("PREVIEW"));
    preview_title.add_css_class("section-title");
    preview_title.set_xalign(0.0);
    body.append(&preview_title);

    let preview_label = Label::new(Some("Select a folder or file to preview actions."));
    preview_label.add_css_class("preview-text");
    preview_label.set_xalign(0.0);
    preview_label.set_wrap(true);
    preview_label.set_selectable(true);
    preview_label.set_margin_top(6);
    preview_label.set_margin_bottom(6);
    preview_label.set_margin_start(8);
    preview_label.set_margin_end(8);

    let preview_scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(false)
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(120)
        .child(&preview_label)
        .build();
    preview_scroller.add_css_class("surface");
    body.append(&preview_scroller);

    let quick_actions = GtkBox::new(Orientation::Horizontal, 8);
    quick_actions.add_css_class("action-row");
    let tree_toggle_btn = Button::with_label("Tree: Off");
    tree_toggle_btn.add_css_class("tree-btn");
    let expand_toggle_btn = Button::with_label("Expand/Collapse");
    expand_toggle_btn.add_css_class("tree-btn");
    let help_btn = Button::with_label("Help");
    let docs_btn = Button::with_label("Docs");
    let guide_btn = Button::with_label("User Guide");
    quick_actions.append(&tree_toggle_btn);
    quick_actions.append(&expand_toggle_btn);
    quick_actions.append(&help_btn);
    quick_actions.append(&docs_btn);
    quick_actions.append(&guide_btn);
    body.append(&quick_actions);

    let terminal = vte4::Terminal::new();
    terminal.set_vexpand(true);
    terminal.set_hexpand(true);
    terminal.set_scrollback_lines(SCROLLBACK_LINES);
    let terminal_font = gtk::pango::FontDescription::from_string("monospace 11");
    terminal.set_font(Some(&terminal_font));

    let terminal_scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&terminal)
        .min_content_height(260)
        .build();
    terminal_scroller.add_css_class("terminal-surface");

    let terminal_title = Label::new(Some("TERMINAL"));
    terminal_title.add_css_class("section-title");
    terminal_title.set_xalign(0.0);
    body.append(&terminal_title);
    body.append(&terminal_scroller);

    let prompt_row = GtkBox::new(Orientation::Horizontal, 8);
    prompt_row.add_css_class("prompt-bar");
    let prompt_label = Label::new(Some(">"));
    prompt_label.add_css_class("prompt-symbol");
    prompt_label.set_margin_start(2);
    prompt_label.set_margin_end(4);

    let command_entry = Entry::new();
    command_entry.add_css_class("prompt-entry");
    command_entry.set_hexpand(true);
    command_entry.set_placeholder_text(Some("Guided prompt (ghost suggestions enabled)"));

    let ghost_label = Label::new(None);
    ghost_label.add_css_class("ghost-text");
    ghost_label.set_xalign(0.0);
    ghost_label.add_css_class("dim-label");

    let mode_label = Label::new(Some("Mode: Guided"));
    mode_label.add_css_class("mode-chip");
    mode_label.set_xalign(1.0);

    prompt_row.append(&prompt_label);
    prompt_row.append(&command_entry);
    prompt_row.append(&ghost_label);
    prompt_row.append(&mode_label);
    body.append(&prompt_row);

    root.append(&body);

    let ui = Ui {
        working_label,
        status_label,
        breadcrumb_bar,
        file_list,
        preview_label,
        tree_toggle_btn,
        expand_toggle_btn,
        command_entry,
        ghost_label,
        mode_label,
        terminal,
    };

    refresh_view(&state, &ui);

    setup_file_navigation(&state, &ui);
    setup_terminal(&state, &ui);
    setup_tree_toggle(&state, &ui);
    setup_expand_toggle(&state, &ui);
    setup_quick_actions(&state, &ui, &help_btn, &docs_btn, &guide_btn);
    setup_prompt_line(&state, &ui);
    start_path_indexer(&state);
    start_status_updater(&state, &ui);

    let state_for_close = state.clone();
    window.connect_close_request(move |_| {
        run_quiet_close_index(&state_for_close);
        glib::Propagation::Proceed
    });

    window.set_child(Some(&root));
    window.present();
}

fn start_status_updater(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let state_for_status = state.clone();
    let ui_for_status = ui.clone();
    glib::timeout_add_seconds_local(1, move || {
        update_status_label(&state_for_status, &ui_for_status);
        glib::ControlFlow::Continue
    });
}

fn mark_activity(state: &Rc<RefCell<AppState>>) {
    state.borrow_mut().last_activity_ts = unix_now();
}

fn queue_path_for_index(state: &Rc<RefCell<AppState>>, path: &Path) {
    let mut app = state.borrow_mut();
    if !app.indexing_enabled {
        return;
    }
    enqueue_index_path(&mut app, path.to_path_buf());
    if let Some(parent) = path.parent() {
        enqueue_index_path(&mut app, parent.to_path_buf());
    }
}

fn schedule_post_navigation_index(state: &Rc<RefCell<AppState>>, dir: &Path) {
    let (indexing_enabled, settings) = {
        let app = state.borrow();
        (app.indexing_enabled, app.index_settings.clone())
    };
    if !indexing_enabled {
        return;
    }

    let target = dir.to_path_buf();
    let state_for_schedule = state.clone();
    glib::timeout_add_local_once(
        std::time::Duration::from_millis(settings.ui_post_nav_delay_ms),
        move || {
            if is_usage_high(settings.high_usage_load_per_cpu) {
                let mut app = state_for_schedule.borrow_mut();
                enqueue_index_path(&mut app, target.clone());
                if settings.ui_post_nav_force_secs > 0 {
                    app.index_force_until_ts =
                        unix_now().saturating_add(settings.ui_post_nav_force_secs);
                }
                return;
            }

            let db_path = state_for_schedule.borrow().index_db_path.clone();
            let conn = match open_index_db(&db_path) {
                Ok(conn) => conn,
                Err(err) => {
                    state_for_schedule.borrow_mut().index_last_error =
                        Some(format!("post-nav index db failed: {err}"));
                    return;
                }
            };

            let mut candidates = Vec::new();
            candidates.push(target.clone());
            if let Ok(iter) = fs::read_dir(&target) {
                for item in iter.flatten().take(settings.ui_post_nav_scan_limit) {
                    candidates.push(item.path());
                }
            }

            let mut queued = 0_usize;
            {
                let mut app = state_for_schedule.borrow_mut();
                for path in candidates {
                    if !path_allowed_for_indexing(&path, &settings) {
                        continue;
                    }
                    if should_index_path(&conn, &path) {
                        enqueue_index_path(&mut app, path);
                        queued += 1;
                    }
                }
                if queued > 0 {
                    app.index_force_until_ts =
                        unix_now().saturating_add(settings.ui_post_nav_force_secs);
                }
            }
        },
    );
}

fn start_path_indexer(state: &Rc<RefCell<AppState>>) {
    let tick_ms = {
        let mut app = state.borrow_mut();
        if !app.indexing_enabled {
            return;
        }
        if app.index_in_progress {
            return;
        }
        app.index_in_progress = true;

        let roots = index_roots(
            &app.current_dir,
            app.home_dir.as_deref(),
            &app.index_settings,
        );
        for root in roots {
            enqueue_index_path(&mut app, root);
        }
        app.index_settings.ui_tick_ms
    };

    let state_for_tick = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(tick_ms), move || {
        index_tick(&state_for_tick);
        glib::ControlFlow::Continue
    });
}

fn index_roots(current_dir: &Path, home_dir: Option<&Path>, settings: &IndexSettings) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut seen = HashSet::new();

    let mut maybe_push = |p: PathBuf| {
        if p.is_dir()
            && path_allowed_for_indexing(&p, settings)
            && seen.insert(p.to_string_lossy().to_string())
        {
            roots.push(p);
        }
    };

    for p in &settings.roots {
        maybe_push(p.to_path_buf());
    }
    maybe_push(current_dir.to_path_buf());
    if let Some(home) = home_dir {
        maybe_push(home.to_path_buf());
    }

    roots
}

fn enqueue_index_path(app: &mut AppState, path: PathBuf) {
    if !app.indexing_enabled || !path_allowed_for_indexing(&path, &app.index_settings) {
        return;
    }
    if app.index_queue.len() >= INDEX_MAX_QUEUE {
        return;
    }
    let key = path.to_string_lossy().to_string();
    if app.index_seen.insert(key) {
        app.index_queue.push_back(path);
    }
}

fn index_tick(state: &Rc<RefCell<AppState>>) {
    let threshold = state.borrow().index_settings.high_usage_load_per_cpu;
    let high_usage = is_usage_high(threshold);
    let (db_path, mut batch, should_index_now, enabled) = {
        let mut app = state.borrow_mut();
        if !app.indexing_enabled {
            app.index_paused_high_usage = false;
            (PathBuf::new(), Vec::new(), false, false)
        } else {
            let db = app.index_db_path.clone();
            let now = unix_now();
            let in_startup =
                now.saturating_sub(app.startup_ts) <= app.index_settings.ui_startup_grace_secs;
            let is_idle =
                now.saturating_sub(app.last_activity_ts) >= app.index_settings.ui_idle_after_secs;
            let force_run = now <= app.index_force_until_ts;
            let should = (in_startup || is_idle || force_run) && !high_usage;
            app.index_paused_high_usage = high_usage && !app.index_queue.is_empty();
            let mut out = Vec::new();
            if should {
                for _ in 0..app.index_settings.ui_batch_limit {
                    let Some(p) = app.index_queue.pop_front() else {
                        break;
                    };
                    let key = p.to_string_lossy().to_string();
                    app.index_seen.remove(&key);
                    out.push(p);
                }
            }
            (db, out, should, true)
        }
    };

    if !enabled {
        return;
    }

    if !should_index_now || batch.is_empty() {
        return;
    }

    let mut conn = match open_index_db(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            state.borrow_mut().index_last_error = Some(format!("open index db failed: {err}"));
            return;
        }
    };

    let now = unix_now();
    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(err) => {
            state.borrow_mut().index_last_error = Some(format!("index tx failed: {err}"));
            return;
        }
    };

    for path in batch.drain(..) {
        let changed = index_one_path(&tx, &path, now);

        if changed {
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if meta.is_dir() {
                    if let Ok(iter) = fs::read_dir(&path) {
                        let mut app = state.borrow_mut();
                        for item in iter.flatten() {
                            enqueue_index_path(&mut app, item.path());
                        }
                    }
                }
            }
        }
    }

    if let Err(err) = tx.commit() {
        state.borrow_mut().index_last_error = Some(format!("index commit failed: {err}"));
    } else {
        state.borrow_mut().index_last_error = None;
    }
}

fn run_quiet_close_index(state: &Rc<RefCell<AppState>>) {
    let (enabled, threshold, batch_limit) = {
        let app = state.borrow();
        (
            app.indexing_enabled,
            app.index_settings.high_usage_load_per_cpu,
            app.index_settings.ui_batch_limit,
        )
    };
    if !enabled {
        return;
    }

    let db_path = state.borrow().index_db_path.clone();
    let mut conn = match open_index_db(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            state.borrow_mut().index_last_error = Some(format!("close index db failed: {err}"));
            return;
        }
    };

    let now = unix_now();
    let high_usage = is_usage_high(threshold);
    let mut remaining = if high_usage {
        (INDEX_CLOSE_BATCH_LIMIT / 8).max(1)
    } else {
        INDEX_CLOSE_BATCH_LIMIT
    };

    while remaining > 0 {
        let mut batch = Vec::new();
        {
            let mut app = state.borrow_mut();
            let pull = remaining.min(batch_limit.max(1));
            for _ in 0..pull {
                let Some(p) = app.index_queue.pop_front() else {
                    break;
                };
                let key = p.to_string_lossy().to_string();
                app.index_seen.remove(&key);
                batch.push(p);
            }
        }

        if batch.is_empty() {
            break;
        }

        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(err) => {
                state.borrow_mut().index_last_error =
                    Some(format!("close index tx failed: {err}"));
                return;
            }
        };

        for path in &batch {
            let _ = index_one_path(&tx, path, now);
        }

        if let Err(err) = tx.commit() {
            state.borrow_mut().index_last_error = Some(format!("close index commit failed: {err}"));
            return;
        }

        remaining = remaining.saturating_sub(batch.len());
    }

    state.borrow_mut().index_last_error = None;
}

fn read_loadavg_one() -> Option<f64> {
    let content = fs::read_to_string("/proc/loadavg").ok()?;
    let first = content.split_whitespace().next()?;
    first.parse::<f64>().ok()
}

fn is_usage_high(threshold: f64) -> bool {
    let load_one = read_loadavg_one().unwrap_or(0.0);
    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1) as f64;
    let load_per_cpu = load_one / cpu_count.max(1.0);
    load_per_cpu >= threshold
}

fn metadata_fingerprint(path: &Path) -> Option<PathFingerprint> {
    let meta = fs::symlink_metadata(path).ok()?;
    let is_dir = if meta.is_dir() { 1_i64 } else { 0_i64 };
    let size = meta.len().min(i64::MAX as u64) as i64;
    let mode = (meta.permissions().mode() & 0o7777) as i64;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .min(i64::MAX as u64) as i64;
    Some(PathFingerprint {
        is_dir,
        size,
        mtime,
        mode,
    })
}

fn query_index_fingerprint(conn: &Connection, path: &Path) -> Result<Option<PathFingerprint>, rusqlite::Error> {
    let path_text = path.to_string_lossy().to_string();
    let mut stmt = conn.prepare(
        "SELECT is_dir, size, mtime, mode FROM paths WHERE path = ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query(params![path_text])?;
    if let Some(row) = rows.next()? {
        let is_dir: i64 = row.get(0)?;
        let size: i64 = row.get(1)?;
        let mtime: i64 = row.get(2)?;
        let mode: i64 = row.get(3)?;
        Ok(Some(PathFingerprint {
            is_dir,
            size,
            mtime,
            mode,
        }))
    } else {
        Ok(None)
    }
}

fn should_index_path(conn: &Connection, path: &Path) -> bool {
    let Some(current) = metadata_fingerprint(path) else {
        return false;
    };

    match query_index_fingerprint(conn, path) {
        Ok(Some(existing)) => existing != current,
        Ok(None) => true,
        Err(_) => true,
    }
}

fn index_one_path(conn: &Connection, path: &Path, indexed_at: u64) -> bool {
    let Some(fp) = metadata_fingerprint(path) else {
        return false;
    };
    let path_text = path.to_string_lossy().to_string();
    let previously_indexed = query_index_fingerprint(conn, path).ok().flatten();
    if previously_indexed == Some(fp) {
        return false;
    }

    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string());
    let ext = lowercase_extension(path).unwrap_or_else(|| "".to_string());
    let indexed_at_db = indexed_at.min(i64::MAX as u64) as i64;

    if conn.execute(
        "INSERT INTO paths(path, name, parent, is_dir, size, mtime, mode, ext, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(path) DO UPDATE SET
            name=excluded.name,
            parent=excluded.parent,
            is_dir=excluded.is_dir,
            size=excluded.size,
            mtime=excluded.mtime,
            mode=excluded.mode,
            ext=excluded.ext,
            indexed_at=excluded.indexed_at",
        params![
            path_text,
            name,
            parent,
            fp.is_dir,
            fp.size,
            fp.mtime,
            fp.mode,
            ext,
            indexed_at_db
        ],
    ).is_err() {
        return false;
    }

    true
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn setup_tree_toggle(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let state_for_toggle = state.clone();
    let ui_for_toggle = ui.clone();

    ui.tree_toggle_btn.connect_clicked(move |button| {
        mark_activity(&state_for_toggle);
        {
            let mut app = state_for_toggle.borrow_mut();
            app.tree_mode = !app.tree_mode;
            if app.tree_mode {
                let current = app.current_dir.to_string_lossy().to_string();
                app.expanded_dirs.insert(current);
            }
            button.set_label(if app.tree_mode { "Tree: On" } else { "Tree: Off" });
        }
        refresh_view(&state_for_toggle, &ui_for_toggle);
    });
}

fn setup_expand_toggle(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let state_for_toggle = state.clone();
    let ui_for_toggle = ui.clone();
    ui.expand_toggle_btn.connect_clicked(move |_| {
        mark_activity(&state_for_toggle);
        let selected = ui_for_toggle.file_list.selected_row();
        let Some(row) = selected else {
            return;
        };
        let idx = row.index();
        if idx < 0 {
            return;
        }

        let maybe_entry = state_for_toggle
            .borrow()
            .entries
            .get(idx as usize)
            .cloned();

        let Some(entry) = maybe_entry else {
            return;
        };
        if entry.kind != EntryKind::Directory {
            return;
        }

        {
            let mut app = state_for_toggle.borrow_mut();
            if !app.tree_mode {
                return;
            }
            let key = entry.path.to_string_lossy().to_string();
            if !app.expanded_dirs.insert(key.clone()) {
                app.expanded_dirs.remove(&key);
            }
        }

        refresh_view(&state_for_toggle, &ui_for_toggle);
    });
}

fn setup_quick_actions(
    state: &Rc<RefCell<AppState>>,
    ui: &Ui,
    help_btn: &Button,
    docs_btn: &Button,
    guide_btn: &Button,
) {
    let state_for_help = state.clone();
    let help_preview = ui.preview_label.clone();
    let help_terminal = ui.terminal.clone();
    help_btn.connect_clicked(move |_| {
        mark_activity(&state_for_help);
        show_help_text(&help_preview);
        help_terminal.feed_child(
            b"printf '\nWhat do you want to do?\n[ Manage files ] [ Install software ]\n[ System info  ] [ Network ]\n[ Manage updates ] [ Advanced ]\n\n'\n",
        );
    });

    let state_for_docs = state.clone();
    let docs_terminal = ui.terminal.clone();
    docs_btn.connect_clicked(move |_| {
        mark_activity(&state_for_docs);
        docs_terminal.feed_child(
            b"if command -v xdg-open >/dev/null 2>&1; then xdg-open docs >/dev/null 2>&1 & else echo 'Open ./docs manually'; fi\n",
        );
    });

    let state_for_guide = state.clone();
    let guide_terminal = ui.terminal.clone();
    guide_btn.connect_clicked(move |_| {
        mark_activity(&state_for_guide);
        guide_terminal.feed_child(
            b"if command -v xdg-open >/dev/null 2>&1; then xdg-open StratOS-Design-Doc-v0.4.md >/dev/null 2>&1 & else echo 'Open StratOS-Design-Doc-v0.4.md manually'; fi\n",
        );
    });
}

fn setup_prompt_line(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let state_for_change = state.clone();
    let ui_for_change = ui.clone();
    ui.command_entry.connect_changed(move |_| {
        mark_activity(&state_for_change);
        let text = ui_for_change.command_entry.text().to_string();
        {
            let mut app = state_for_change.borrow_mut();
            if app.ghost_dismissed_for.as_deref() != Some(text.as_str()) {
                app.ghost_dismissed_for = None;
            }
        }
        refresh_ghost(&state_for_change, &ui_for_change);
    });

    let state_for_activate = state.clone();
    let ui_for_activate = ui.clone();
    ui.command_entry.connect_activate(move |_| {
        execute_prompt_command(&state_for_activate, &ui_for_activate);
    });

    let state_for_keys = state.clone();
    let ui_for_keys = ui.clone();
    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        mark_activity(&state_for_keys);
        if matches!(key, gdk::Key::Tab | gdk::Key::Right) {
            if accept_current_ghost(&state_for_keys, &ui_for_keys) {
                return glib::Propagation::Stop;
            }
        }

        if key == gdk::Key::Escape {
            if dismiss_current_ghost(&state_for_keys, &ui_for_keys) {
                return glib::Propagation::Stop;
            }
        }

        glib::Propagation::Proceed
    });
    ui.command_entry.add_controller(key_controller);
}

fn execute_prompt_command(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    mark_activity(state);
    let input = ui.command_entry.text().to_string();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return;
    }

    if trimmed.eq_ignore_ascii_case("help") {
        show_help_text(&ui.preview_label);
        record_command_history(state, "help".to_string());
        clear_prompt_line(state, ui);
        return;
    }

    if trimmed.eq_ignore_ascii_case("advanced") {
        ui.terminal.feed_child(
            b"echo 'Dropping to full shell. Type exit to come back.'\n",
        );
        {
            let mut app = state.borrow_mut();
            app.advanced_mode = true;
        }
        ui.mode_label.set_text("Mode: Advanced");
        record_command_history(state, "advanced".to_string());
        clear_prompt_line(state, ui);
        return;
    }

    if trimmed.eq_ignore_ascii_case("exit") {
        let was_advanced = state.borrow().advanced_mode;
        if was_advanced {
            {
                let mut app = state.borrow_mut();
                app.advanced_mode = false;
            }
            ui.mode_label.set_text("Mode: Guided");
            ui.terminal.feed_child(b"echo 'Returned to guided mode.'\n");
            record_command_history(state, "exit".to_string());
            clear_prompt_line(state, ui);
            return;
        }
    }

    let is_advanced = state.borrow().advanced_mode;
    if !is_advanced {
        if let Some(mapped) = map_guided_intent(trimmed) {
            let line = format!("{mapped}\n");
            ui.terminal.feed_child(line.as_bytes());
            record_command_history(state, mapped);
            clear_prompt_line(state, ui);
            return;
        }
    }

    let command = expand_special_command(state, trimmed);
    let line = format!("{command}\n");
    ui.terminal.feed_child(line.as_bytes());
    record_command_history(state, command);
    clear_prompt_line(state, ui);
}

fn clear_prompt_line(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    ui.command_entry.set_text("");
    ui.ghost_label.set_text("");

    let mut app = state.borrow_mut();
    app.current_ghost = None;
    app.ghost_dismissed_for = None;
}

fn show_help_text(label: &Label) {
    label.set_text(
        "What do you want to do?\n\n[ Manage files ]      [ Install software ]\n[ System info  ]      [ Network ]\n[ Manage updates ]    [ Advanced ]\n\nGuided examples:\n- how much ram\n- disk space\n- what's running\n- install ripgrep\n- connect to wifi\n\nOr just type shell commands.",
    );
}

fn map_guided_intent(input: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    let trimmed = input.trim();

    if lower.contains("how much ram") || lower == "ram" || lower.contains("memory usage") {
        return Some("free -h".to_string());
    }

    if lower.contains("disk space") || lower.contains("disk usage") {
        return Some("df -h".to_string());
    }

    if lower.contains("what's running")
        || lower.contains("whats running")
        || lower.contains("running processes")
        || lower.contains("process list")
    {
        return Some("ps aux".to_string());
    }

    if let Some(rest) = trimmed.strip_prefix("install ") {
        let pkg = rest.trim();
        if !pkg.is_empty() {
            return Some(format!("strat install {}", shell_quote(pkg)));
        }
    }

    if lower.contains("connect to wifi") || lower == "wifi" || lower.contains("wireless") {
        return Some(
            "if command -v nmcli >/dev/null 2>&1; then nmcli device wifi list; else echo 'nmcli unavailable'; fi"
                .to_string(),
        );
    }

    if lower.contains("system info") {
        return Some("uname -a; free -h; df -h".to_string());
    }

    if lower == "docs" {
        return Some(
            "if command -v xdg-open >/dev/null 2>&1; then xdg-open docs >/dev/null 2>&1 & else echo 'Open ./docs manually'; fi"
                .to_string(),
        );
    }

    if lower == "user guide" || lower == "guide" {
        return Some(
            "if command -v xdg-open >/dev/null 2>&1; then xdg-open StratOS-Design-Doc-v0.4.md >/dev/null 2>&1 & else echo 'Open StratOS-Design-Doc-v0.4.md manually'; fi"
                .to_string(),
        );
    }

    None
}

fn accept_current_ghost(state: &Rc<RefCell<AppState>>, ui: &Ui) -> bool {
    let ghost = state.borrow().current_ghost.clone();
    let Some(ghost) = ghost else {
        return false;
    };
    let current = ui.command_entry.text().to_string();
    if !ghost.starts_with(&current) {
        return false;
    }

    ui.command_entry.set_text(&ghost);
    ui.command_entry.set_position(-1);

    {
        let mut app = state.borrow_mut();
        app.current_ghost = Some(ghost.clone());
        app.ghost_dismissed_for = None;
    }

    // Ghost acceptance is local to the prompt line only.
    // We do not send any shell input here; execution still requires Enter.
    ui.ghost_label.set_text("");

    true
}

fn dismiss_current_ghost(state: &Rc<RefCell<AppState>>, ui: &Ui) -> bool {
    let input = ui.command_entry.text().to_string();
    if input.is_empty() {
        return false;
    }

    ui.ghost_label.set_text("");

    let mut app = state.borrow_mut();
    app.current_ghost = None;
    app.ghost_dismissed_for = Some(input);
    true
}

fn refresh_ghost(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let input = ui.command_entry.text().to_string();
    if input.trim().is_empty() {
        ui.ghost_label.set_text("");
        let mut app = state.borrow_mut();
        app.current_ghost = None;
        app.ghost_dismissed_for = None;
        return;
    }

    {
        let app = state.borrow();
        if app.ghost_dismissed_for.as_deref() == Some(input.as_str()) {
            ui.ghost_label.set_text("");
            drop(app);
            state.borrow_mut().current_ghost = None;
            return;
        }
    }

    let ghost = {
        let app = state.borrow();
        compute_ghost_command(&app, &input)
    };

    {
        let mut app = state.borrow_mut();
        app.current_ghost = ghost
            .clone()
            .filter(|suggestion| suggestion.starts_with(&input));
    }

    if let Some(ghost_text) = ghost.filter(|suggestion| suggestion.starts_with(&input)) {
        if let Some(suffix) = ghost_text.strip_prefix(&input) {
            ui.ghost_label.set_text(suffix);
        }
    } else {
        ui.ghost_label.set_text("");
    }
}

fn compute_ghost_command(state: &AppState, input: &str) -> Option<String> {
    let trimmed = input.trim_start();

    if let Some(token) = trimmed.strip_prefix("cd -s ") {
        let token = token.trim();
        if token.is_empty() {
            return None;
        }

        return expand_smart_cd_token(state, token).map(|path| {
            let quoted = shell_quote(path.to_string_lossy().as_ref());
            format!("cd {quoted}")
        });
    }

    if let Some(fragment) = trimmed.strip_prefix("cd ") {
        let fragment = fragment.trim();
        if fragment.is_empty() {
            return None;
        }

        return suggest_cd_path(state, fragment).map(|path| {
            let quoted = shell_quote(path.to_string_lossy().as_ref());
            format!("cd {quoted}")
        });
    }

    suggest_command_history(state, input)
}

fn suggest_command_history(state: &AppState, input: &str) -> Option<String> {
    let needle = input.to_ascii_lowercase();

    for command in state.command_history.iter().rev() {
        let lower = command.to_ascii_lowercase();
        if lower.starts_with(&needle) && command != input {
            return Some(command.clone());
        }
    }

    None
}

fn suggest_cd_path(state: &AppState, fragment: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for (path_text, frec) in &state.frecency {
        let path = PathBuf::from(path_text);
        if !path.is_dir() {
            continue;
        }
        if let Some(score) = cd_match_score(fragment, &path, state.home_dir.as_deref()) {
            if seen.insert(path_text.clone()) {
                candidates.push(CdCandidate {
                    path,
                    source: CandidateSource::Frecency,
                    score,
                    count: frec.count,
                    last_visit: frec.last_visit,
                });
            }
        }
    }

    let (cwd_dirs, _) = read_sorted_entries(&state.current_dir);
    for path in cwd_dirs {
        let key = path.to_string_lossy().to_string();
        if seen.contains(&key) {
            continue;
        }
        if let Some(score) = cd_match_score(fragment, &path, state.home_dir.as_deref()) {
            seen.insert(key);
            candidates.push(CdCandidate {
                path,
                source: CandidateSource::Cwd,
                score,
                count: 0,
                last_visit: 0,
            });
        }
    }

    for path in index_dir_candidates(state, fragment, 800) {
        let key = path.to_string_lossy().to_string();
        if seen.contains(&key) {
            continue;
        }
        if let Some(score) = cd_match_score(fragment, &path, state.home_dir.as_deref()) {
            seen.insert(key);
            candidates.push(CdCandidate {
                path,
                source: CandidateSource::Home,
                score,
                count: 0,
                last_visit: 0,
            });
        }
    }

    for path in &state.system_path_dirs {
        let key = path.to_string_lossy().to_string();
        if seen.contains(&key) {
            continue;
        }
        if let Some(score) = cd_match_score(fragment, path, state.home_dir.as_deref()) {
            seen.insert(key);
            candidates.push(CdCandidate {
                path: path.clone(),
                source: CandidateSource::System,
                score,
                count: 0,
                last_visit: 0,
            });
        }
    }

    candidates.sort_by(compare_cd_candidates);
    candidates.into_iter().next().map(|candidate| candidate.path)
}

fn index_dir_candidates(state: &AppState, fragment: &str, limit: usize) -> Vec<PathBuf> {
    let conn = match open_index_db(&state.index_db_path) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };

    let needle = fragment.trim();
    if needle.is_empty() {
        return Vec::new();
    }

    let mut name_like = needle.to_string();
    name_like.push('%');
    let mut path_like = "%".to_string();
    path_like.push_str(needle);
    path_like.push('%');

    let mut stmt = match conn.prepare(
        "SELECT path FROM paths
         WHERE is_dir = 1 AND (name LIKE ?1 COLLATE NOCASE OR path LIKE ?2 COLLATE NOCASE)
         ORDER BY indexed_at DESC
         LIMIT ?3",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };

    let lim = limit.min(i64::MAX as usize) as i64;
    let rows = match stmt.query_map(params![name_like, path_like, lim], |row| {
        let p: String = row.get(0)?;
        Ok(p)
    }) {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };

    rows.flatten().map(PathBuf::from).collect()
}

fn compare_cd_candidates(a: &CdCandidate, b: &CdCandidate) -> Ordering {
    let source_rank = |source: CandidateSource| match source {
        CandidateSource::Frecency => 0_u8,
        CandidateSource::Cwd => 1,
        CandidateSource::Home => 2,
        CandidateSource::System => 3,
    };

    let rank_cmp = source_rank(a.source).cmp(&source_rank(b.source));
    if rank_cmp != Ordering::Equal {
        return rank_cmp;
    }

    if a.source == CandidateSource::Frecency {
        let recent_cmp = b.last_visit.cmp(&a.last_visit);
        if recent_cmp != Ordering::Equal {
            return recent_cmp;
        }

        let freq_cmp = b.count.cmp(&a.count);
        if freq_cmp != Ordering::Equal {
            return freq_cmp;
        }
    }

    let score_cmp = a.score.cmp(&b.score);
    if score_cmp != Ordering::Equal {
        return score_cmp;
    }

    let len_cmp = a.path.as_os_str().len().cmp(&b.path.as_os_str().len());
    if len_cmp != Ordering::Equal {
        return len_cmp;
    }

    a.path.cmp(&b.path)
}

fn cd_match_score(fragment: &str, path: &Path, home_dir: Option<&Path>) -> Option<u64> {
    let needle = fragment.to_ascii_lowercase();

    let full_display = display_path_for_user(path, home_dir).to_ascii_lowercase();
    let base_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_default();

    if full_display.starts_with(&needle) || base_name.starts_with(&needle) {
        return Some(0);
    }

    if is_subsequence(&needle, &base_name) {
        return Some(1);
    }

    if full_display.contains(&needle) || base_name.contains(&needle) {
        return Some(2);
    }

    None
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    let mut chars = needle.chars();
    let mut current = chars.next();

    for c in haystack.chars() {
        if Some(c) == current {
            current = chars.next();
            if current.is_none() {
                return true;
            }
        }
    }

    false
}

fn expand_special_command(state: &Rc<RefCell<AppState>>, command: &str) -> String {
    if let Some(token) = command.strip_prefix("cd -s ") {
        let token = token.trim();
        if token.is_empty() {
            return command.to_string();
        }

        let expanded = {
            let app = state.borrow();
            expand_smart_cd_token(&app, token)
        };

        if let Some(path) = expanded {
            let quoted = shell_quote(path.to_string_lossy().as_ref());
            return format!("cd {quoted}");
        }
    }

    command.to_string()
}

fn expand_smart_cd_token(state: &AppState, token: &str) -> Option<PathBuf> {
    let (abbr, suffix) = match token.split_once('/') {
        Some((prefix, rest)) => (prefix, Some(rest)),
        None => (token, None),
    };

    let abbr = abbr.trim();
    if abbr.is_empty() {
        return None;
    }

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for (path_text, frec) in &state.frecency {
        let path = PathBuf::from(path_text);
        if !path.is_dir() {
            continue;
        }

        if smart_abbr_matches(abbr, &path, state.home_dir.as_deref())
            && seen.insert(path_text.clone())
        {
            candidates.push(CdCandidate {
                path,
                source: CandidateSource::Frecency,
                score: 0,
                count: frec.count,
                last_visit: frec.last_visit,
            });
        }
    }

    for path in index_dir_candidates(state, abbr, 1200) {
        let key = path.to_string_lossy().to_string();
        if seen.contains(&key) {
            continue;
        }

        if smart_abbr_matches(abbr, &path, state.home_dir.as_deref()) {
            seen.insert(key);
            candidates.push(CdCandidate {
                path,
                source: CandidateSource::Home,
                score: 0,
                count: 0,
                last_visit: 0,
            });
        }
    }

    candidates.sort_by(compare_cd_candidates);
    let best = candidates.into_iter().next().map(|candidate| candidate.path)?;

    if let Some(suffix) = suffix {
        let joined = best.join(suffix);
        return Some(normalize_dir(joined));
    }

    Some(best)
}

fn smart_abbr_matches(abbr: &str, path: &Path, home_dir: Option<&Path>) -> bool {
    let initials = path_initials(path, home_dir);
    initials.starts_with(&abbr.to_ascii_lowercase())
}

fn path_initials(path: &Path, home_dir: Option<&Path>) -> String {
    let target = if let Some(home) = home_dir {
        path.strip_prefix(home).unwrap_or(path)
    } else {
        path
    };

    let mut initials = String::new();
    for part in target.components() {
        if let Component::Normal(name) = part {
            let s = name.to_string_lossy();
            if let Some(first) = s.chars().next() {
                initials.push(first.to_ascii_lowercase());
            }
        }
    }

    initials
}

fn setup_terminal(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let shell = preferred_shell();
    let cwd = state.borrow().current_dir.clone();
    let cwd_str = cwd.to_string_lossy().to_string();

    let terminal = ui.terminal.clone();
    let state_for_spawn = state.clone();
    let argv = [shell.as_str(), "-i"];
    let envv = ["TERM=xterm-256color", "COLORTERM=truecolor"];

    terminal.spawn_async(
        vte4::PtyFlags::DEFAULT,
        Some(&cwd_str),
        &argv,
        &envv,
        glib::SpawnFlags::DEFAULT,
        || {},
        -1,
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok(pid) => {
                state_for_spawn.borrow_mut().child_pid = Some(pid);
            }
            Err(err) => {
                eprintln!("stratterm: failed to spawn shell: {err}");
            }
        },
    );

    let state_for_poll = state.clone();
    let ui_for_poll = ui.clone();
    // CWD sync source-of-truth policy:
    // We intentionally use /proc/<shell-pid>/cwd polling as the single authority.
    // This avoids split-brain behavior between OSC7 and process-state tracking.
    glib::timeout_add_seconds_local(1, move || {
        let child_pid = state_for_poll.borrow().child_pid;
        if let Some(pid) = child_pid {
            let cwd_path = format!("/proc/{pid}/cwd");
            if let Ok(dir) = fs::read_link(&cwd_path) {
                change_directory(&state_for_poll, &ui_for_poll, dir, false);
            }
        }
        glib::ControlFlow::Continue
    });
}

fn setup_file_navigation(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let state_for_select = state.clone();
    let preview_for_select = ui.preview_label.clone();
    ui.file_list.connect_row_selected(move |_list, row| {
        mark_activity(&state_for_select);
        let Some(row) = row else {
            return;
        };
        let idx = row.index();
        if idx < 0 {
            return;
        }

        let entry = state_for_select.borrow().entries.get(idx as usize).cloned();
        if let Some(entry) = entry {
            preview_for_select.set_text(&preview_text_for_entry(&entry));
            queue_path_for_index(&state_for_select, &entry.path);
            let entry_key = entry.path.to_string_lossy().to_string();
            let mut app = state_for_select.borrow_mut();
            if app.pending_script_confirmation.as_deref() != Some(entry_key.as_str()) {
                app.pending_script_confirmation = None;
            }
        }
    });

    let state_for_activate = state.clone();
    let ui_for_activate = ui.clone();
    ui.file_list.connect_row_activated(move |_list, row| {
        mark_activity(&state_for_activate);
        let idx = row.index();
        if idx < 0 {
            return;
        }

        let entry = state_for_activate
            .borrow()
            .entries
            .get(idx as usize)
            .cloned();

        let Some(entry) = entry else {
            return;
        };

        match entry.kind {
            EntryKind::Up | EntryKind::Directory => {
                change_directory(&state_for_activate, &ui_for_activate, entry.path, true);
            }
            EntryKind::File => {
                run_file_activation(&state_for_activate, &ui_for_activate, &entry.path);
            }
        }
    });
}

fn run_file_activation(state: &Rc<RefCell<AppState>>, ui: &Ui, path: &Path) {
    mark_activity(state);
    queue_path_for_index(state, path);
    let path_text = path.to_string_lossy();
    let path_quoted = shell_quote(path_text.as_ref());

    if is_executable(path) && !is_script_file(path) {
        state.borrow_mut().pending_script_confirmation = None;
        let command = format!(
            "echo 'Refusing to auto-run executable: {p}'; echo 'Run it manually if trusted: {p}'\n",
            p = path_quoted
        );
        ui.terminal.feed_child(command.as_bytes());
        return;
    }

    if is_script_file(path) {
        let key = path.to_string_lossy().to_string();
        let should_run = {
            let mut app = state.borrow_mut();
            if app.pending_script_confirmation.as_deref() == Some(key.as_str()) {
                app.pending_script_confirmation = None;
                true
            } else {
                app.pending_script_confirmation = Some(key.clone());
                false
            }
        };

        if !should_run {
            ui.preview_label.set_text(&format!(
                "Script confirmation required:\n{}\n\nDouble-click the same script again to run it.",
                path.display()
            ));
            let prompt = format!(
                "echo 'Script requires confirmation. Double-click again to run: {p}'\n",
                p = path_quoted
            );
            ui.terminal.feed_child(prompt.as_bytes());
            return;
        }

        let runner = script_runner(path);
        let command = if runner.is_empty() {
            format!("{path_quoted}\n")
        } else {
            format!("{runner} {path_quoted}\n")
        };
        ui.terminal.feed_child(command.as_bytes());
        return;
    }

    state.borrow_mut().pending_script_confirmation = None;
    if is_config_file(path) {
        let command = format!(
            "if command -v nano >/dev/null 2>&1; then nano {p}; elif command -v vi >/dev/null 2>&1; then vi {p}; else less {p}; fi\n",
            p = path_quoted
        );
        ui.terminal.feed_child(command.as_bytes());
        return;
    }

    let command = format!(
        "if command -v xdg-open >/dev/null 2>&1; then xdg-open {p} >/dev/null 2>&1 & else less {p}; fi\n",
        p = path_quoted
    );
    ui.terminal.feed_child(command.as_bytes());
}

fn script_runner(path: &Path) -> &'static str {
    if is_executable(path) {
        return "";
    }

    match lowercase_extension(path).as_deref() {
        Some("sh") => "bash",
        Some("py") => "python3",
        Some("pl") => "perl",
        Some("rb") => "ruby",
        _ => "bash",
    }
}

fn refresh_view(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    rebuild_file_list(state, ui);
    rebuild_breadcrumbs(state, ui);

    let app = state.borrow();
    ui.working_label
        .set_text(&format!("Working in: {}", app.current_dir.display()));
    ui.tree_toggle_btn
        .set_label(if app.tree_mode { "Tree: On" } else { "Tree: Off" });
    ui.expand_toggle_btn.set_sensitive(app.tree_mode);
    if app.advanced_mode {
        ui.mode_label.set_text("Mode: Advanced");
        ui.command_entry
            .set_placeholder_text(Some("Advanced shell command"));
    } else {
        ui.mode_label.set_text("Mode: Guided");
        ui.command_entry
            .set_placeholder_text(Some("Guided prompt (ghost suggestions enabled)"));
    }
    ui.preview_label.set_text(&format!(
        "Directory view ready. Mode: {}.\nSingle-click to preview, double-click to open/navigate.",
        if app.tree_mode { "Tree" } else { "Flat" }
    ));

    drop(app);
    update_status_label(state, ui);
}

fn update_status_label(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    let app = state.borrow();
    let mode = if app.advanced_mode { "Advanced" } else { "Guided" };
    let view = if app.tree_mode { "Tree" } else { "Flat" };
    let item_count = app.entries.len();
    let index_state = if !app.indexing_enabled {
        "Index disabled".to_string()
    } else if app.index_last_error.is_some() {
        "Indexer error".to_string()
    } else if app.index_paused_high_usage && !app.index_queue.is_empty() {
        "Index paused (high usage)".to_string()
    } else if !app.index_queue.is_empty() {
        format!("Index {} queued", app.index_queue.len())
    } else {
        "Index ready".to_string()
    };
    ui.status_label.set_text(&format!(
        "{mode} · {view} · {item_count} items · {index_state}"
    ));
}

fn rebuild_file_list(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    clear_children(&ui.file_list);

    let app = state.borrow();
    let current_dir = app.current_dir.clone();
    let tree_mode = app.tree_mode;
    drop(app);

    let mut entries = Vec::new();

    let parent = current_dir
        .parent()
        .map_or_else(|| PathBuf::from("/"), Path::to_path_buf);
    entries.push(BrowserEntry {
        path: parent,
        kind: EntryKind::Up,
        depth: 0,
        expanded: false,
        tree_hint: false,
    });

    if tree_mode {
        let expanded_dirs = state.borrow().expanded_dirs.clone();
        append_tree_entries(
            &current_dir,
            0,
            MAX_TREE_DEPTH,
            &expanded_dirs,
            &mut entries,
        );
    } else {
        append_flat_entries(&current_dir, 0, &mut entries);
    }

    for entry in &entries {
        let row = ListBoxRow::new();
        row.set_activatable(true);
        row.add_css_class("entry-row");
        match entry.kind {
            EntryKind::Up => row.add_css_class("entry-up"),
            EntryKind::Directory => row.add_css_class("entry-dir"),
            EntryKind::File => row.add_css_class("entry-file"),
        }

        let label = Label::new(Some(&entry_title(entry)));
        label.add_css_class("entry-label");
        label.set_xalign(0.0);
        label.set_margin_start(8);
        label.set_margin_end(8);
        label.set_margin_top(6);
        label.set_margin_bottom(6);

        row.set_child(Some(&label));
        ui.file_list.append(&row);
    }

    state.borrow_mut().entries = entries;
}

fn append_flat_entries(dir: &Path, depth: usize, out: &mut Vec<BrowserEntry>) {
    let (dirs, files) = read_sorted_entries(dir);

    for path in dirs {
        out.push(BrowserEntry {
            path,
            kind: EntryKind::Directory,
            depth,
            expanded: false,
            tree_hint: false,
        });
    }

    for path in files {
        out.push(BrowserEntry {
            path,
            kind: EntryKind::File,
            depth,
            expanded: false,
            tree_hint: false,
        });
    }
}

fn append_tree_entries(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    expanded_dirs: &HashSet<String>,
    out: &mut Vec<BrowserEntry>,
) {
    let (dirs, files) = read_sorted_entries(dir);

    for path in dirs {
        let key = path.to_string_lossy().to_string();
        let is_expanded = expanded_dirs.contains(&key);
        out.push(BrowserEntry {
            path: path.clone(),
            kind: EntryKind::Directory,
            depth,
            expanded: is_expanded,
            tree_hint: true,
        });
        if depth < max_depth && is_expanded {
            append_tree_entries(&path, depth + 1, max_depth, expanded_dirs, out);
        }
    }

    for path in files {
        out.push(BrowserEntry {
            path,
            kind: EntryKind::File,
            depth,
            expanded: false,
            tree_hint: true,
        });
    }
}

fn read_sorted_entries(dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    if let Ok(iter) = fs::read_dir(dir) {
        for item in iter.flatten() {
            let path = item.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.push(path);
            }
        }
    }

    dirs.sort_by_key(|path| file_name_lower(path));
    files.sort_by_key(|path| file_name_lower(path));
    (dirs, files)
}

fn entry_title(entry: &BrowserEntry) -> String {
    let indent = "  ".repeat(entry.depth);
    match entry.kind {
        EntryKind::Up => format!("{indent}DIR  .. (go up)"),
        EntryKind::Directory => {
            let name = entry
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("(unknown)");
            if entry.tree_hint {
                let marker = if entry.expanded { "[-]" } else { "[+]" };
                format!("{indent}DIR  {marker} {name}/")
            } else {
                format!("{indent}DIR  {name}/")
            }
        }
        EntryKind::File => {
            let name = entry
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("(unknown)");
            format!("{indent}FILE {name}")
        }
    }
}

fn file_name_lower(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_ascii_lowercase())
        .unwrap_or_else(|| path.to_string_lossy().to_ascii_lowercase())
}

fn rebuild_breadcrumbs(state: &Rc<RefCell<AppState>>, ui: &Ui) {
    clear_box_children(&ui.breadcrumb_bar);

    let path = state.borrow().current_dir.clone();

    let state_for_root = state.clone();
    let ui_for_root = ui.clone();
    let root_btn = Button::with_label("/");
    root_btn.add_css_class("tree-btn");
    root_btn.connect_clicked(move |_| {
        change_directory(&state_for_root, &ui_for_root, PathBuf::from("/"), true);
    });
    ui.breadcrumb_bar.append(&root_btn);

    let mut accumulated = PathBuf::from("/");
    for component in path.components() {
        if matches!(component, Component::RootDir) {
            continue;
        }

        let part = component.as_os_str().to_string_lossy().to_string();
        accumulated.push(&part);

        let slash = Label::new(Some("/"));
        ui.breadcrumb_bar.append(&slash);

        let target = accumulated.clone();
        let state_for_btn = state.clone();
        let ui_for_btn = ui.clone();
        let button = Button::with_label(&part);
        button.add_css_class("tree-btn");
        button.connect_clicked(move |_| {
            change_directory(&state_for_btn, &ui_for_btn, target.clone(), true);
        });
        ui.breadcrumb_bar.append(&button);
    }
}

fn preview_text_for_entry(entry: &BrowserEntry) -> String {
    match entry.kind {
        EntryKind::Up => format!("Navigate up to: {}", entry.path.display()),
        EntryKind::Directory => preview_directory(&entry.path),
        EntryKind::File => preview_file(&entry.path),
    }
}

fn preview_directory(path: &Path) -> String {
    let (dirs, files) = read_sorted_entries(path);
    let mut lines = Vec::new();
    lines.push(format!("Folder: {}", path.display()));
    lines.push(format!("Contains: {} folder(s), {} file(s)", dirs.len(), files.len()));

    for dir in dirs.iter().take(6) {
        lines.push(format!("  DIR  {}/", file_name_lower(dir)));
    }
    for file in files.iter().take(6) {
        lines.push(format!("  FILE {}", file_name_lower(file)));
    }

    if dirs.len() + files.len() > 12 {
        lines.push("  ...".to_string());
    }

    lines.push("Double-click to navigate into this folder.".to_string());
    lines.join("\n")
}

fn preview_file(path: &Path) -> String {
    let metadata = fs::metadata(path).ok();
    let mut lines = Vec::new();
    lines.push(format!("File: {}", path.display()));

    if let Some(meta) = metadata {
        lines.push(format!("Size: {} bytes", meta.len()));
        if meta.permissions().mode() & 0o111 != 0 {
            lines.push("Type: executable".to_string());
        }
    }

    if is_executable(path) && !is_script_file(path) {
        lines.push("Safety: executable detected; double-click will NOT auto-run.".to_string());
        lines.push("Run it manually in terminal if trusted.".to_string());
        return lines.join("\n");
    }

    if is_script_file(path) {
        lines.push(
            "Behavior: script file, double-click once to arm, double-click again to run."
                .to_string(),
        );
        lines.extend(read_text_preview(path, MAX_PREVIEW_LINES));
        return lines.join("\n");
    }

    if is_config_file(path) {
        lines.push("Behavior: config file, double-click opens in editor.".to_string());
        lines.push(config_summary(path));
        lines.extend(read_text_preview(path, 6));
        return lines.join("\n");
    }

    lines.push("Behavior: normal file, double-click opens in default app.".to_string());
    lines.extend(read_text_preview(path, MAX_PREVIEW_LINES));
    lines.join("\n")
}

fn read_text_preview(path: &Path, max_lines: usize) -> Vec<String> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(err) => {
            return vec![format!("Preview unavailable: {err}")];
        }
    };

    let mut buf = vec![0_u8; 4096];
    let count = match file.read(&mut buf) {
        Ok(count) => count,
        Err(err) => {
            return vec![format!("Preview unavailable: {err}")];
        }
    };

    buf.truncate(count);

    if buf.contains(&0) {
        return vec!["Binary file detected; text preview omitted.".to_string()];
    }

    let text = String::from_utf8_lossy(&buf);
    let mut lines = vec!["Preview:".to_string()];
    for line in text.lines().take(max_lines) {
        lines.push(format!("  {line}"));
    }
    lines
}

fn config_summary(path: &Path) -> String {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return "Config summary unavailable (non-text or unreadable).".to_string(),
    };

    let line_count = text.lines().count();
    let key_like = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with(';')
        })
        .filter(|line| line.contains('=') || line.contains(':'))
        .count();

    format!(
        "Config summary: about {line_count} line(s), {key_like} key/value-style entries."
    )
}

fn is_script_file(path: &Path) -> bool {
    match lowercase_extension(path).as_deref() {
        Some("sh") | Some("py") | Some("pl") | Some("rb") | Some("lua") | Some("js") => {
            true
        }
        _ => has_shebang(path),
    }
}

fn is_config_file(path: &Path) -> bool {
    matches!(
        lowercase_extension(path).as_deref(),
        Some("conf")
            | Some("cfg")
            | Some("ini")
            | Some("toml")
            | Some("yaml")
            | Some("yml")
            | Some("json")
            | Some("xml")
            | Some("env")
            | Some("service")
    )
}

fn lowercase_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

fn has_shebang(path: &Path) -> bool {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let mut buf = [0_u8; 2];
    match file.read(&mut buf) {
        Ok(2) => buf == [b'#', b'!'],
        _ => false,
    }
}

fn change_directory(state: &Rc<RefCell<AppState>>, ui: &Ui, dir: PathBuf, push_to_shell: bool) {
    mark_activity(state);
    let normalized = normalize_dir(dir);
    if !normalized.is_dir() {
        return;
    }

    let should_refresh = {
        let mut app = state.borrow_mut();
        if app.current_dir == normalized {
            false
        } else {
            app.current_dir = normalized.clone();
            record_directory_visit(&mut app, &normalized);
            true
        }
    };

    if should_refresh {
        state.borrow_mut().pending_script_confirmation = None;
        refresh_view(state, ui);
        schedule_post_navigation_index(state, &normalized);
    }

    if push_to_shell {
        let command = format!("cd {}\n", shell_quote(normalized.to_string_lossy().as_ref()));
        ui.terminal.feed_child(command.as_bytes());
    }
}

fn normalize_dir(path: PathBuf) -> PathBuf {
    let absolute = if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };

    fs::canonicalize(&absolute).unwrap_or(absolute)
}

fn preferred_shell() -> String {
    if command_exists("fish") {
        return "fish".to_string();
    }
    if command_exists("bash") {
        return "bash".to_string();
    }
    "/bin/sh".to_string()
}

fn command_exists(command: &str) -> bool {
    if command.contains('/') {
        return is_executable(Path::new(command));
    }

    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path_var)
        .map(|dir| dir.join(command))
        .any(|candidate| is_executable(&candidate))
}

fn is_executable(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_file() && (metadata.permissions().mode() & 0o111 != 0),
        Err(_) => false,
    }
}

fn shell_quote(raw: &str) -> String {
    if raw.is_empty() {
        return "''".to_string();
    }

    if raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'-' | b'~'))
    {
        return raw.to_string();
    }

    format!("'{}'", raw.replace('\'', "'\"'\"'"))
}

fn record_command_history(state: &Rc<RefCell<AppState>>, command: String) {
    let mut app = state.borrow_mut();
    app.command_history.push(command);
    if app.command_history.len() > 1000 {
        let drain = app.command_history.len() - 1000;
        app.command_history.drain(0..drain);
    }
}

fn frecency_db_path(home_dir: Option<&Path>) -> PathBuf {
    if let Some(home) = home_dir {
        return home.join(".config/strat/frecency.db");
    }

    PathBuf::from("/tmp/stratterm_frecency.db")
}

fn index_db_path(home_dir: Option<&Path>) -> PathBuf {
    if Path::new("/config").is_dir() {
        return PathBuf::from("/config/strat/index.db");
    }

    if let Some(home) = home_dir {
        return home.join(".config/strat/index.db");
    }

    PathBuf::from("/tmp/stratterm_index.db")
}

fn open_index_db(path: &Path) -> Result<Connection, rusqlite::Error> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS paths (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            parent TEXT NOT NULL,
            is_dir INTEGER NOT NULL,
            size INTEGER NOT NULL,
            mtime INTEGER NOT NULL,
            mode INTEGER NOT NULL,
            ext TEXT NOT NULL,
            indexed_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_paths_name ON paths(name);
         CREATE INDEX IF NOT EXISTS idx_paths_parent ON paths(parent);
         CREATE INDEX IF NOT EXISTS idx_paths_mtime ON paths(mtime DESC);",
    )?;
    Ok(conn)
}

fn load_frecency(path: &Path) -> HashMap<String, FrecencyRecord> {
    let mut data = HashMap::new();
    let Ok(conn) = open_frecency_db(path) else {
        return data;
    };

    let mut stmt = match conn.prepare("SELECT path, count, last_visit FROM frecency") {
        Ok(stmt) => stmt,
        Err(err) => {
            eprintln!(
                "stratterm: could not read frecency table {}: {err}",
                path.display()
            );
            return data;
        }
    };

    let rows = match stmt.query_map([], |row| {
        let path_text: String = row.get(0)?;
        let count_db: i64 = row.get(1)?;
        let last_visit_db: i64 = row.get(2)?;
        let count = count_db.max(0) as u64;
        let last_visit = last_visit_db.max(0) as u64;
        Ok((path_text, FrecencyRecord { count, last_visit }))
    }) {
        Ok(rows) => rows,
        Err(err) => {
            eprintln!(
                "stratterm: could not query frecency rows {}: {err}",
                path.display()
            );
            return data;
        }
    };

    for row in rows.flatten() {
        data.insert(row.0, row.1);
    }

    data
}

fn open_frecency_db(path: &Path) -> Result<Connection, rusqlite::Error> {
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!(
                "stratterm: could not create frecency dir {}: {err}",
                parent.display()
            );
        }
    }

    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS frecency (
            path TEXT PRIMARY KEY,
            count INTEGER NOT NULL,
            last_visit INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_frecency_last_visit
            ON frecency(last_visit DESC);",
    )?;
    Ok(conn)
}

fn save_frecency_visit(path: &Path, dir_path: &str, last_visit: u64) {
    let Ok(conn) = open_frecency_db(path) else {
        return;
    };

    let last_visit_db = last_visit.min(i64::MAX as u64) as i64;
    if let Err(err) = conn.execute(
        "INSERT INTO frecency(path, count, last_visit)
         VALUES (?1, 1, ?2)
         ON CONFLICT(path) DO UPDATE SET
             count = count + 1,
             last_visit = excluded.last_visit",
        params![dir_path, last_visit_db],
    ) {
        eprintln!(
            "stratterm: could not persist frecency visit {}: {err}",
            path.display()
        );
    }
}

fn record_directory_visit(app: &mut AppState, path: &Path) {
    let key = path.to_string_lossy().to_string();
    let now = unix_timestamp();

    let record = app.frecency.entry(key).or_default();
    record.count = record.count.saturating_add(1);
    record.last_visit = now;

    save_frecency_visit(&app.frecency_db_path, key.as_str(), now);
}

fn unix_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

fn collect_system_path_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let Some(path_var) = env::var_os("PATH") else {
        return out;
    };

    for path in env::split_paths(&path_var) {
        if !path.is_dir() {
            continue;
        }

        let key = path.to_string_lossy().to_string();
        if seen.insert(key) {
            out.push(path);
        }
    }

    out
}

fn display_path_for_user(path: &Path, home_dir: Option<&Path>) -> String {
    if let Some(home) = home_dir {
        if let Ok(rest) = path.strip_prefix(home) {
            if rest.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rest.display());
        }
    }

    path.to_string_lossy().to_string()
}

fn clear_children(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn clear_box_children(container: &GtkBox) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}
