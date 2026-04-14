use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Adjustment, Application, ApplicationWindow, Box as GtkBox, Button, Entry, Grid, Image, Label,
    Orientation, PolicyType, ScrolledWindow, SpinButton, Stack, Switch,
};
use std::env;
use std::path::{Path, PathBuf};
use stratterm::index_settings::{
    config_path, disable_flag_exists, load_index_settings, save_index_settings, set_disable_flag,
    IndexSettings,
};

const APP_ID: &str = "org.stratos.StratSettings";

#[derive(Clone)]
struct SettingsWidgets {
    enabled: Switch,
    boot_start: Switch,
    ui_enabled: Switch,
    frequency_ms: SpinButton,
    rescan_secs: SpinButton,
    batch_limit: SpinButton,
    high_usage_load_per_cpu: SpinButton,
    roots: Entry,
    exclude_prefixes: Entry,
    ui_tick_ms: SpinButton,
    ui_batch_limit: SpinButton,
    ui_idle_after_secs: SpinButton,
    ui_startup_grace_secs: SpinButton,
    ui_post_nav_delay_ms: SpinButton,
    ui_post_nav_scan_limit: SpinButton,
    ui_post_nav_force_secs: SpinButton,
    disable_flag: Switch,
    config_label: Label,
    status_label: Label,
}

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let home_dir = env::var_os("HOME").map(PathBuf::from);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("StratOS Settings")
        .default_width(980)
        .default_height(760)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let overview = build_overview_page(&stack);
    let terminal = build_terminal_settings_page(&stack, home_dir);

    stack.add_named(&overview, Some("overview"));
    stack.add_named(&terminal, Some("terminal-indexer"));
    stack.set_visible_child_name("overview");

    root.append(&stack);
    window.set_child(Some(&root));
    window.present();
}

fn build_overview_page(stack: &Stack) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 10);
    page.set_margin_top(10);
    page.set_margin_bottom(10);
    page.set_margin_start(10);
    page.set_margin_end(10);

    let title = Label::new(Some("System Settings"));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    page.append(&title);

    let subtitle = Label::new(Some(
        "Choose a settings panel. Terminal settings are nested under the terminal icon.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    page.append(&subtitle);

    let system_header = Label::new(Some("System"));
    system_header.set_xalign(0.0);
    system_header.add_css_class("title-4");
    page.append(&system_header);

    let system_grid = Grid::builder()
        .column_spacing(14)
        .row_spacing(14)
        .hexpand(true)
        .build();

    let terminal_tile = icon_tile(
        "Terminal",
        "Indexer & Shell",
        "utilities-terminal-symbolic",
        true,
    );
    {
        let stack_for_click = stack.clone();
        terminal_tile.connect_clicked(move |_| {
            stack_for_click.set_visible_child_name("terminal-indexer");
        });
    }
    system_grid.attach(&terminal_tile, 0, 0, 1, 1);

    let storage_tile = icon_tile("Storage", "Coming Soon", "drive-harddisk-symbolic", false);
    system_grid.attach(&storage_tile, 1, 0, 1, 1);

    let display_tile = icon_tile("Display", "Coming Soon", "video-display-symbolic", false);
    system_grid.attach(&display_tile, 2, 0, 1, 1);

    let network_tile = icon_tile("Network", "Coming Soon", "network-workgroup-symbolic", false);
    system_grid.attach(&network_tile, 3, 0, 1, 1);

    let updates_tile = icon_tile("Updates", "Coming Soon", "software-update-available-symbolic", false);
    system_grid.attach(&updates_tile, 4, 0, 1, 1);

    page.append(&system_grid);

    let apps_header = Label::new(Some("Apps"));
    apps_header.set_xalign(0.0);
    apps_header.add_css_class("title-4");
    apps_header.set_margin_top(10);
    page.append(&apps_header);

    let apps_grid = Grid::builder()
        .column_spacing(14)
        .row_spacing(14)
        .hexpand(true)
        .build();

    let files_tile = icon_tile("Files", "Coming Soon", "folder-symbolic", false);
    apps_grid.attach(&files_tile, 0, 0, 1, 1);

    let compositor_tile = icon_tile("Compositor", "Coming Soon", "preferences-system-symbolic", false);
    apps_grid.attach(&compositor_tile, 1, 0, 1, 1);

    let privacy_tile = icon_tile("Privacy", "Coming Soon", "changes-prevent-symbolic", false);
    apps_grid.attach(&privacy_tile, 2, 0, 1, 1);

    page.append(&apps_grid);
    page
}

fn icon_tile(title: &str, subtitle: &str, icon_name: &str, enabled: bool) -> Button {
    let button = Button::new();
    button.set_size_request(150, 118);

    let content = GtkBox::new(Orientation::Vertical, 6);
    content.set_margin_top(10);
    content.set_margin_bottom(10);
    content.set_margin_start(8);
    content.set_margin_end(8);

    let icon = Image::from_icon_name(icon_name);
    icon.set_pixel_size(36);
    content.append(&icon);

    let title_label = Label::new(Some(title));
    title_label.set_xalign(0.5);
    title_label.add_css_class("heading");
    content.append(&title_label);

    let subtitle_label = Label::new(Some(subtitle));
    subtitle_label.set_xalign(0.5);
    subtitle_label.add_css_class("caption");
    content.append(&subtitle_label);

    button.set_child(Some(&content));
    button.set_sensitive(enabled);
    button
}

fn build_terminal_settings_page(stack: &Stack, home_dir: Option<PathBuf>) -> GtkBox {
    let settings = load_index_settings(home_dir.as_deref());
    let disable_active = disable_flag_exists(home_dir.as_deref());

    let page = GtkBox::new(Orientation::Vertical, 10);
    page.set_margin_top(10);
    page.set_margin_bottom(10);
    page.set_margin_start(10);
    page.set_margin_end(10);

    let top_bar = GtkBox::new(Orientation::Horizontal, 8);
    let back_btn = Button::with_label("Show All");
    {
        let stack_for_back = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_for_back.set_visible_child_name("overview");
        });
    }
    let title = Label::new(Some("Terminal Settings"));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);

    top_bar.append(&back_btn);
    top_bar.append(&title);
    page.append(&top_bar);

    let subtitle = Label::new(Some(
        "Configure terminal indexer backend behavior. This panel maps to terminal/indexer config.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    page.append(&subtitle);

    let hint = Label::new(Some("Tip: hover each setting label or input for a quick explanation."));
    hint.set_xalign(0.0);
    hint.add_css_class("caption");
    page.append(&hint);

    let config_label = Label::new(Some(&format!(
        "Config file: {}",
        config_path(home_dir.as_deref()).display()
    )));
    config_label.set_xalign(0.0);
    page.append(&config_label);

    let scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .build();
    let grid = Grid::builder()
        .column_spacing(16)
        .row_spacing(8)
        .hexpand(true)
        .vexpand(true)
        .build();

    let widgets = build_controls(&grid, &settings, disable_active, config_label.clone());
    scroller.set_child(Some(&grid));
    page.append(&scroller);

    let buttons = GtkBox::new(Orientation::Horizontal, 8);
    let reload_btn = Button::with_label("Reload");
    let save_btn = Button::with_label("Save");
    buttons.append(&reload_btn);
    buttons.append(&save_btn);
    page.append(&buttons);
    page.append(&widgets.status_label);

    let home_for_reload = home_dir.clone();
    let widgets_for_reload = widgets.clone();
    reload_btn.connect_clicked(move |_| {
        let loaded = load_index_settings(home_for_reload.as_deref());
        let disable = disable_flag_exists(home_for_reload.as_deref());
        apply_to_widgets(&widgets_for_reload, &loaded, disable, home_for_reload.as_deref());
        widgets_for_reload
            .status_label
            .set_text("Reloaded settings from disk.");
    });

    let home_for_save = home_dir;
    let widgets_for_save = widgets.clone();
    save_btn.connect_clicked(move |_| {
        let mut settings = collect_from_widgets(&widgets_for_save);
        if settings.roots.is_empty() {
            settings.roots = IndexSettings::default().roots;
        }

        let saved_to = save_index_settings(&settings, home_for_save.as_deref());
        let disable_result = set_disable_flag(
            widgets_for_save.disable_flag.is_active(),
            home_for_save.as_deref(),
        );

        match (saved_to, disable_result) {
            (Ok(saved_path), Ok(_)) => {
                widgets_for_save
                    .config_label
                    .set_text(&format!("Config file: {}", saved_path.display()));
                widgets_for_save.status_label.set_text(
                    "Saved settings. Restart indexer/terminal process to guarantee full apply.",
                );
            }
            (Err(err), Ok(_)) => {
                widgets_for_save
                    .status_label
                    .set_text(&format!("Save failed: {err}"));
            }
            (Ok(saved_path), Err(flag_err)) => {
                widgets_for_save
                    .config_label
                    .set_text(&format!("Config file: {}", saved_path.display()));
                widgets_for_save
                    .status_label
                    .set_text(&format!("Saved config, disable flag update failed: {flag_err}"));
            }
            (Err(err), Err(flag_err)) => {
                widgets_for_save.status_label.set_text(&format!(
                    "Save failed: {err}. Disable flag update failed: {flag_err}"
                ));
            }
        }
    });

    page
}

fn build_controls(
    grid: &Grid,
    settings: &IndexSettings,
    disable_active: bool,
    config_label: Label,
) -> SettingsWidgets {
    let enabled = Switch::builder().active(settings.enabled).build();
    add_row_with_tooltip(
        grid,
        0,
        "Enable indexing",
        "Master switch for all indexing behavior in StratTerm and background indexer.",
        &enabled,
    );

    let boot_start = Switch::builder().active(settings.boot_start).build();
    add_row_with_tooltip(
        grid,
        1,
        "Start indexer on boot",
        "When enabled, the background indexer daemon starts during boot.",
        &boot_start,
    );

    let ui_enabled = Switch::builder().active(settings.ui_enabled).build();
    add_row_with_tooltip(
        grid,
        2,
        "Enable UI-side indexer",
        "Controls indexing work triggered from the StratTerm UI itself.",
        &ui_enabled,
    );

    let disable_flag = Switch::builder().active(disable_active).build();
    add_row_with_tooltip(
        grid,
        3,
        "Hard disable flag (/config/strat/disable-indexer)",
        "Creates/removes a hard-disable flag file. If enabled, indexing is forced off.",
        &disable_flag,
    );

    let frequency_ms = spin_u64(settings.frequency_ms as f64, 100.0, 60000.0, 100.0);
    add_row_with_tooltip(
        grid,
        4,
        "Daemon frequency (ms)",
        "Base sleep interval between daemon work loops. Lower is more responsive but costs more CPU.",
        &frequency_ms,
    );

    let rescan_secs = spin_u64(settings.rescan_secs as f64, 10.0, 86400.0, 10.0);
    add_row_with_tooltip(
        grid,
        5,
        "Daemon rescan interval (sec)",
        "How often the daemon re-enqueues top-level roots for change checks.",
        &rescan_secs,
    );

    let batch_limit = spin_u64(settings.batch_limit as f64, 1.0, 10000.0, 1.0);
    add_row_with_tooltip(
        grid,
        6,
        "Daemon batch limit",
        "Maximum number of queued paths processed per daemon transaction.",
        &batch_limit,
    );

    let high_usage_load_per_cpu = spin_f64(settings.high_usage_load_per_cpu, 0.10, 4.00, 0.05, 2);
    add_row_with_tooltip(
        grid,
        7,
        "High-load pause threshold (load/core)",
        "Indexing pauses when (1-minute load average / CPU cores) is above this value.",
        &high_usage_load_per_cpu,
    );

    let roots = Entry::new();
    roots.set_text(&join_paths(&settings.roots));
    add_row_with_tooltip(
        grid,
        8,
        "Index roots (comma-separated)",
        "Absolute path prefixes allowed for indexing. Example: /home,/config,/apps",
        &roots,
    );

    let exclude_prefixes = Entry::new();
    exclude_prefixes.set_text(&join_paths(&settings.exclude_prefixes));
    add_row_with_tooltip(
        grid,
        9,
        "Exclude prefixes (comma-separated)",
        "Absolute path prefixes skipped even if under roots.",
        &exclude_prefixes,
    );

    let ui_tick_ms = spin_u64(settings.ui_tick_ms as f64, 100.0, 10000.0, 50.0);
    add_row_with_tooltip(
        grid,
        10,
        "UI tick (ms)",
        "Interval for UI-side index worker polling while StratTerm is open.",
        &ui_tick_ms,
    );

    let ui_batch_limit = spin_u64(settings.ui_batch_limit as f64, 1.0, 10000.0, 1.0);
    add_row_with_tooltip(
        grid,
        11,
        "UI batch limit",
        "Maximum number of paths UI-side indexing handles per tick.",
        &ui_batch_limit,
    );

    let ui_idle_after_secs = spin_u64(settings.ui_idle_after_secs as f64, 1.0, 86400.0, 1.0);
    add_row_with_tooltip(
        grid,
        12,
        "UI idle after (sec)",
        "How long input must be idle before normal UI-side indexing starts.",
        &ui_idle_after_secs,
    );

    let ui_startup_grace_secs =
        spin_u64(settings.ui_startup_grace_secs as f64, 0.0, 86400.0, 1.0);
    add_row_with_tooltip(
        grid,
        13,
        "UI startup grace (sec)",
        "Short startup window where UI-side indexing is allowed before idle rules apply.",
        &ui_startup_grace_secs,
    );

    let ui_post_nav_delay_ms = spin_u64(settings.ui_post_nav_delay_ms as f64, 0.0, 10000.0, 25.0);
    add_row_with_tooltip(
        grid,
        14,
        "UI post-nav delay (ms)",
        "Delay after folder navigation before post-navigation indexing attempts.",
        &ui_post_nav_delay_ms,
    );

    let ui_post_nav_scan_limit =
        spin_u64(settings.ui_post_nav_scan_limit as f64, 10.0, 100000.0, 10.0);
    add_row_with_tooltip(
        grid,
        15,
        "UI post-nav scan limit",
        "Maximum children sampled for post-navigation change checks.",
        &ui_post_nav_scan_limit,
    );

    let ui_post_nav_force_secs =
        spin_u64(settings.ui_post_nav_force_secs as f64, 0.0, 3600.0, 1.0);
    add_row_with_tooltip(
        grid,
        16,
        "UI post-nav force window (sec)",
        "Temporary window that prioritizes queued post-navigation indexing work.",
        &ui_post_nav_force_secs,
    );

    let status_label = Label::new(Some("Ready."));
    status_label.set_xalign(0.0);

    SettingsWidgets {
        enabled,
        boot_start,
        ui_enabled,
        frequency_ms,
        rescan_secs,
        batch_limit,
        high_usage_load_per_cpu,
        roots,
        exclude_prefixes,
        ui_tick_ms,
        ui_batch_limit,
        ui_idle_after_secs,
        ui_startup_grace_secs,
        ui_post_nav_delay_ms,
        ui_post_nav_scan_limit,
        ui_post_nav_force_secs,
        disable_flag,
        config_label,
        status_label,
    }
}

fn apply_to_widgets(
    widgets: &SettingsWidgets,
    settings: &IndexSettings,
    disable_active: bool,
    home_dir: Option<&Path>,
) {
    widgets.enabled.set_active(settings.enabled);
    widgets.boot_start.set_active(settings.boot_start);
    widgets.ui_enabled.set_active(settings.ui_enabled);
    widgets.frequency_ms.set_value(settings.frequency_ms as f64);
    widgets.rescan_secs.set_value(settings.rescan_secs as f64);
    widgets.batch_limit.set_value(settings.batch_limit as f64);
    widgets
        .high_usage_load_per_cpu
        .set_value(settings.high_usage_load_per_cpu);
    widgets.roots.set_text(&join_paths(&settings.roots));
    widgets
        .exclude_prefixes
        .set_text(&join_paths(&settings.exclude_prefixes));
    widgets.ui_tick_ms.set_value(settings.ui_tick_ms as f64);
    widgets.ui_batch_limit.set_value(settings.ui_batch_limit as f64);
    widgets
        .ui_idle_after_secs
        .set_value(settings.ui_idle_after_secs as f64);
    widgets
        .ui_startup_grace_secs
        .set_value(settings.ui_startup_grace_secs as f64);
    widgets
        .ui_post_nav_delay_ms
        .set_value(settings.ui_post_nav_delay_ms as f64);
    widgets
        .ui_post_nav_scan_limit
        .set_value(settings.ui_post_nav_scan_limit as f64);
    widgets
        .ui_post_nav_force_secs
        .set_value(settings.ui_post_nav_force_secs as f64);
    widgets.disable_flag.set_active(disable_active);
    widgets
        .config_label
        .set_text(&format!("Config file: {}", config_path(home_dir).display()));
}

fn collect_from_widgets(w: &SettingsWidgets) -> IndexSettings {
    IndexSettings {
        enabled: w.enabled.is_active(),
        boot_start: w.boot_start.is_active(),
        frequency_ms: w.frequency_ms.value().round().max(100.0) as u64,
        rescan_secs: w.rescan_secs.value().round().max(10.0) as u64,
        batch_limit: w.batch_limit.value().round().clamp(1.0, 10000.0) as usize,
        high_usage_load_per_cpu: w.high_usage_load_per_cpu.value().clamp(0.10, 4.0),
        roots: parse_path_list(w.roots.text().as_str()),
        exclude_prefixes: parse_path_list(w.exclude_prefixes.text().as_str()),
        ui_enabled: w.ui_enabled.is_active(),
        ui_tick_ms: w.ui_tick_ms.value().round().max(100.0) as u64,
        ui_batch_limit: w.ui_batch_limit.value().round().clamp(1.0, 10000.0) as usize,
        ui_idle_after_secs: w.ui_idle_after_secs.value().round().max(1.0) as u64,
        ui_startup_grace_secs: w.ui_startup_grace_secs.value().round().max(0.0) as u64,
        ui_post_nav_delay_ms: w.ui_post_nav_delay_ms.value().round().max(0.0) as u64,
        ui_post_nav_scan_limit: w
            .ui_post_nav_scan_limit
            .value()
            .round()
            .clamp(10.0, 100000.0) as usize,
        ui_post_nav_force_secs: w.ui_post_nav_force_secs.value().round().max(0.0) as u64,
    }
}

fn parse_path_list(raw: &str) -> Vec<PathBuf> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn join_paths(paths: &[PathBuf]) -> String {
    let mut out = Vec::new();
    for p in paths {
        out.push(p.to_string_lossy().to_string());
    }
    out.join(",")
}

fn add_row_with_tooltip<T: IsA<gtk::Widget>>(
    grid: &Grid,
    row: i32,
    label_text: &str,
    tooltip: &str,
    widget: &T,
) {
    let label = Label::new(Some(label_text));
    label.set_xalign(0.0);
    label.set_tooltip_text(Some(tooltip));
    widget.set_tooltip_text(Some(tooltip));
    grid.attach(&label, 0, row, 1, 1);
    grid.attach(widget, 1, row, 1, 1);
}

fn spin_u64(value: f64, min: f64, max: f64, step: f64) -> SpinButton {
    let adj = Adjustment::new(value, min, max, step, step * 10.0, 0.0);
    SpinButton::builder().adjustment(&adj).digits(0).build()
}

fn spin_f64(value: f64, min: f64, max: f64, step: f64, digits: u32) -> SpinButton {
    let adj = Adjustment::new(value, min, max, step, step * 10.0, 0.0);
    SpinButton::builder()
        .adjustment(&adj)
        .digits(digits)
        .build()
}
